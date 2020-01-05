// This file is part of Stampede.
//
// Stampede is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// Stampede is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with Stampede.  If not, see <http://www.gnu.org/licenses/>.
use lazy_static::lazy_static;
use rand::prelude::*;
use std::{f32::consts::PI, mem};
use wgpu;

pub const INSTRUCTION_COUNT: usize = 128;
pub const CONSTANT_POOL_SIZE: usize = 1024;

pub struct InstructionEncoder {
    instrs: [u32; INSTRUCTION_COUNT],
    instr_offset: usize,

    constant_pool: [f32; CONSTANT_POOL_SIZE],
    pool_offset: usize,
}

impl InstructionEncoder {
    pub fn instruction_buffer_size() -> wgpu::BufferAddress {
        mem::size_of::<[u64; INSTRUCTION_COUNT]>() as wgpu::BufferAddress
    }

    pub fn pool_buffer_size() -> wgpu::BufferAddress {
        mem::size_of::<[f32; CONSTANT_POOL_SIZE]>() as wgpu::BufferAddress
    }

    pub fn new() -> Self {
        Self {
            instrs: [0u32; INSTRUCTION_COUNT],
            instr_offset: 0,
            constant_pool: [0f32; CONSTANT_POOL_SIZE],
            pool_offset: 0,
        }
    }

    pub fn finish(self) -> ([u32; INSTRUCTION_COUNT], [f32; CONSTANT_POOL_SIZE]) {
        (self.instrs, self.constant_pool)
    }

    pub fn push<Op: Opcode>(&mut self, op: &Op) {
        let children = op.get_children();
        let consts = op.get_constants();
        for child in children {
            child.encode(self);
        }
        for v in consts {
            self.push_constant(v.value());
        }
        let op_bits = ((consts.len() & 0xFF) as u32) << 16
            | ((children.len() & 0xFF) as u32) << 8
            | (Op::opcode() as u32);
        self.instrs[self.instr_offset] = op_bits;
        self.instr_offset += 1;
    }

    pub fn push_constant(&mut self, value: f32) {
        self.constant_pool[self.pool_offset] = value;
        self.pool_offset += 1;
    }
}

pub trait Opcode {
    fn opcode() -> usize;
    fn get_constants(&self) -> &[Constant];
    fn get_children(&self) -> &[Box<Node>];
}

fn prefix(level: usize) -> String {
    let mut s = String::new();
    for _ in 0..level {
        s += " ";
    }
    s
}

#[derive(Debug, Eq, PartialEq)]
pub enum WrapMode {
    Repeat,
    Mirror,
}

impl WrapMode {
    pub fn from_name(name: &'static str) -> Self {
        match name {
            "m" => Self::Mirror,
            "r" => Self::Repeat,
            "f" => Self::Repeat, // "fixed" does not wrap, so we can pick anything
            _ => panic!("Unknown wrap mode name"),
        }
    }
}

pub const RATE_SCALE: f32 = 500f32;

#[derive(Debug)]
pub struct Constant {
    limits: [f32; 2],
    value: f32,
    rate: f32,
    wrap_mode: WrapMode,
}

impl Constant {
    pub fn new(rng: &mut StdRng, min_bound: f32, max_bound: f32, mode_name: &'static str) -> Self {
        let rate = if mode_name != "f" {
            rng.gen_range(min_bound / RATE_SCALE, max_bound / RATE_SCALE)
        } else {
            0f32
        };
        Self {
            limits: [min_bound, max_bound],
            value: rng.gen_range(min_bound, max_bound),
            rate,
            wrap_mode: WrapMode::from_name(mode_name),
        }
    }

    pub fn value(&self) -> f32 {
        self.value
    }

    pub fn animate(&mut self) {
        self.value += self.rate;
        if self.value < self.limits[0] {
            match self.wrap_mode {
                WrapMode::Repeat => self.value += (self.limits[1] - self.limits[0]),
                WrapMode::Mirror => {
                    self.value = self.limits[0] + (self.limits[0] - self.value);
                    self.rate *= -1f32;
                }
            }
        }
        if self.value > self.limits[1] {
            match self.wrap_mode {
                WrapMode::Repeat => self.value -= (self.limits[1] - self.limits[0]),
                WrapMode::Mirror => {
                    self.value = self.limits[1] - (self.value - self.limits[1]);
                    self.rate *= -1f32;
                }
            }
        }
    }
}

macro_rules! make_op {
    ($op_name:ident [$opcode:literal] {
        constants($const_count:literal) => [$($const_name:ident[$min_bound:expr,$max_bound:expr,$wrap_mode:ident]),*],
        children($child_count:literal) => [$($child_name:ident),*]
    }) => {
        #[derive(Debug)]
        pub struct $op_name {
            consts: [Constant; $const_count],
            children: [Box<Node>; $child_count]
        }

        impl $op_name {
            pub fn new(rng: &mut StdRng, _count: &mut usize) -> Self {
                Self {
                    consts: [
                        $(
                            Constant::new(rng, ($min_bound) as f32, ($max_bound) as f32, stringify!($wrap_mode))
                            //rng.gen_range(($min_bound) as f32, ($max_bound) as f32)
                        ),*
                    ],
                    children: [
                        $(
                            Box::new(Node::new(rng, _count, stringify!($child_name)))
                        ),*
                    ],
                }
            }

            pub fn animate(&mut self) {
                for child in self.children.iter_mut() {
                    child.animate();
                }
                for c in self.consts.iter_mut() {
                    c.animate();
                }
            }
            /*
            #[allow(dead_code)]
            pub fn with_constants($($const_name: f32),*) -> Self {
                let _rng = &mut thread_rng();
                let _count = &mut 0;
                Self {
                    consts: [
                        Constant::new(_rng, -1f32, 1f32, "m"),
                    ],
                    children: [
                        $(
                            Box::new(Node::new(_rng, _count, stringify!($child_name)))
                        ),*
                    ],
                }
            }
            */

            pub fn show(&self, level: usize) -> String {
                let cc = self.consts.iter().map(|v| format!("{:0.2}", v.value())).collect::<Vec<String>>().join(", ");
                if $child_count == 0 {
                    format!("{}{}({})", prefix(level), stringify!($op_name), cc)
                } else {
                    let ch = self.children.iter().map(|c| c.show(level + 1)).collect::<Vec<String>>().join("\n");
                    format!("{}{}({})-\n{}", prefix(level), stringify!($op_name), cc, ch)
                }
            }
        }

        impl Opcode for $op_name {
            fn opcode() -> usize {
                $opcode
            }

            fn get_constants(&self) -> &[Constant] {
                &self.consts
            }

            fn get_children(&self) -> &[Box<Node>] {
                &self.children
            }
        }
    }
}

make_op!(ConstOp          [1] { constants(1) => [value[-1,1,m]], children(0) => [] });
make_op!(EllipseOp        [2] { constants(6) => [p0x[-1,1,m], p0y[-0.8,0.8,m], p1x[-1,1,m], p1y[-0.8,0.8,m], size[0.1,1,m], sharp[1,100,m]], children(0) => [] });
make_op!(FlowerOp         [3] { constants(7) => [x[-1,1,m], y[-0.8,0.8,m], angle[0,2.0*PI,r], size[0,2.5,m], ratio[0,1,m], n_points[3,25,f], sharpness[2,10,m]], children(0) => [] });
make_op!(LinearGradientOp [4] { constants(5) => [p0x[-1,1,m], p0y[-0.8,0.8,m], p1x[-1,1,m], p1y[-0.8,0.8,m], sharp[2,20,m]], children(0) => [] });
make_op!(RadialGradientOp [5] { constants(5) => [p0x[-1,1,m], p0y[-0.8,0.8,m], p1x[-1,1,m], p1y[-0.8,0.8,m], angle[0,2.0*PI,r]], children(0) => [] });
make_op!(PolarThetaOp     [6] { constants(3) => [x[-1,1,m], y[-0.8,0.8,m], angle[0,2.0*PI,r]], children(0) => [] });
//
make_op!(AbsoluteOp       [8] { constants(0) => [], children(1) => [value] });
make_op!(InvertOp         [9] { constants(0) => [], children(1) => [value] });
make_op!(AddOp           [10] { constants(0) => [], children(2) => [lhs, rhs] });
make_op!(SubtractOp      [11] { constants(0) => [], children(2) => [lhs, rhs] });
make_op!(MultiplyOp      [12] { constants(0) => [], children(2) => [lhs, rhs] });
make_op!(DivideOp        [13] { constants(0) => [], children(2) => [lhs, rhs] });
make_op!(ModulusOp       [14] { constants(0) => [], children(2) => [lhs, rhs] });
make_op!(ExponentOp      [15] { constants(0) => [], children(2) => [lhs, rhs] });
make_op!(SincOp          [16] { constants(2) => [freq[-PI,PI,r], phase[-PI,PI,r]], children(1) => [input] });
make_op!(SineOp          [17] { constants(2) => [freq[-PI,PI,r], phase[-PI,PI,r]], children(1) => [input] });
make_op!(SpiralOp        [18] { constants(4) => [x[-1,1,m], y[-0.8,0.8,m], n[0,10,m], b[-1,1,m]], children(1) => [V] });
make_op!(SquircleOp      [19] { constants(4) => [x[-1,1,m], y[-0.8,0.8,m], r[0,2,m], n[0,4,m]], children(2) => [a, b] });

#[derive(Debug)]
pub enum Node {
    // Leaves
    Const(ConstOp),
    Ellipse(EllipseOp),
    Flower(FlowerOp),
    LinearGradient(LinearGradientOp),
    RadialGradient(RadialGradientOp),
    PolarTheta(PolarThetaOp),

    // Operations
    Absolute(AbsoluteOp),
    Invert(InvertOp),
    Add(AddOp),
    Subtract(SubtractOp),
    Multiply(MultiplyOp),
    Divide(DivideOp),
    Modulus(ModulusOp),
    Exponent(ExponentOp),
    Sinc(SincOp),
    Sine(SineOp),
    Spiral(SpiralOp),
    Squircle(SquircleOp),
}

lazy_static! {
    static ref LEAF_RATE_TOTAL: f32 = {
        let mut total = 0.0;
        for (rate, _, _) in &LEAF_RATES {
            total += rate;
        }
        total
    };
    static ref OP_RATE_TOTAL: f32 = {
        let mut total = 0.0;
        for (rate, _, _) in &OP_RATES {
            total += rate;
        }
        total
    };
}

const LEAF_RATES: [(f32, usize, &'static str); 6] = [
    (0.01, 1, "const"),
    (2.00, 2, "ellipse"),
    (4.00, 3, "flower"),
    (1.00, 4, "linear gradient"),
    (2.00, 5, "radial gradient"),
    (2.00, 6, "polar theta"),
];

const OP_RATES: [(f32, usize, &'static str); 12] = [
    (0.2, 8, "absolute"),
    (0.1, 9, "invert"),
    (0.3, 10, "add"),
    (0.3, 11, "subtract"),
    (0.3, 12, "multiply"),
    (0.3, 13, "divide"),
    (0.5, 14, "modulus"),
    (0.5, 15, "exponentiate"),
    (0.0, 16, "sinc"),
    (0.0, 17, "sine"),
    (0.2, 18, "spiral"),
    (2.0, 19, "squircle"),
];

fn guided_random_walk(rng: &mut StdRng, rates: &[(f32, usize, &'static str)], total: f32) -> usize {
    let f = rng.gen_range(0f32, total);
    let mut i = 0;
    let mut acc = 0f32;
    while acc <= f {
        // Note that the interval is half open, so this will always be true.
        acc += rates[i].0;
        i += 1;
    }
    i -= 1; // Hence we can subtract safely here.
    rates[i].1
}

impl Node {
    fn new(rng: &mut StdRng, count: &mut usize, _link_name: &str) -> Self {
        // FIXME: pick a better walk for this
        let fullness = (*count * 2) as f32 / INSTRUCTION_COUNT as f32;
        *count += 1;
        if rng.gen_range(0f32, 1f32) < fullness {
            let x = guided_random_walk(rng, &LEAF_RATES, *LEAF_RATE_TOTAL);
            match x {
                1 => Self::Const(ConstOp::new(rng, count)),
                2 => Self::Ellipse(EllipseOp::new(rng, count)),
                3 => Self::Flower(FlowerOp::new(rng, count)),
                4 => Self::LinearGradient(LinearGradientOp::new(rng, count)),
                5 => Self::RadialGradient(RadialGradientOp::new(rng, count)),
                6 => Self::PolarTheta(PolarThetaOp::new(rng, count)),
                _ => panic!("unknown const opcode"),
            }
        } else {
            let x = guided_random_walk(rng, &OP_RATES, *OP_RATE_TOTAL);
            match x {
                8 => Self::Absolute(AbsoluteOp::new(rng, count)),
                9 => Self::Invert(InvertOp::new(rng, count)),
                10 => Self::Add(AddOp::new(rng, count)),
                11 => Self::Subtract(SubtractOp::new(rng, count)),
                12 => Self::Multiply(MultiplyOp::new(rng, count)),
                13 => Self::Divide(DivideOp::new(rng, count)),
                14 => Self::Modulus(ModulusOp::new(rng, count)),
                15 => Self::Exponent(ExponentOp::new(rng, count)),
                16 => Self::Sinc(SincOp::new(rng, count)),
                17 => Self::Sine(SineOp::new(rng, count)),
                18 => Self::Spiral(SpiralOp::new(rng, count)),
                19 => Self::Squircle(SquircleOp::new(rng, count)),
                _ => panic!("unknown opcode"),
            }
        }
    }

    fn show(&self, level: usize) -> String {
        let l = level + 1;
        match self {
            Self::Const(ref op) => op.show(l),
            Self::Ellipse(ref op) => op.show(l),
            Self::Flower(ref op) => op.show(l),
            Self::LinearGradient(ref op) => op.show(l),
            Self::RadialGradient(ref op) => op.show(l),
            Self::PolarTheta(ref op) => op.show(l),
            Self::Absolute(ref op) => op.show(l),
            Self::Invert(ref op) => op.show(l),
            Self::Add(ref op) => op.show(l),
            Self::Subtract(ref op) => op.show(l),
            Self::Multiply(ref op) => op.show(l),
            Self::Divide(ref op) => op.show(l),
            Self::Modulus(ref op) => op.show(l),
            Self::Exponent(ref op) => op.show(l),
            Self::Sinc(ref op) => op.show(l),
            Self::Sine(ref op) => op.show(l),
            Self::Spiral(ref op) => op.show(l),
            Self::Squircle(ref op) => op.show(l),
        }
    }

    fn encode(&self, encoder: &mut InstructionEncoder) {
        match self {
            Self::Const(ref op) => encoder.push(op),
            Self::Ellipse(ref op) => encoder.push(op),
            Self::Flower(ref op) => encoder.push(op),
            Self::LinearGradient(ref op) => encoder.push(op),
            Self::RadialGradient(ref op) => encoder.push(op),
            Self::PolarTheta(ref op) => encoder.push(op),
            Self::Absolute(ref op) => encoder.push(op),
            Self::Invert(ref op) => encoder.push(op),
            Self::Add(ref op) => encoder.push(op),
            Self::Subtract(ref op) => encoder.push(op),
            Self::Multiply(ref op) => encoder.push(op),
            Self::Divide(ref op) => encoder.push(op),
            Self::Modulus(ref op) => encoder.push(op),
            Self::Exponent(ref op) => encoder.push(op),
            Self::Sinc(ref op) => encoder.push(op),
            Self::Sine(ref op) => encoder.push(op),
            Self::Spiral(ref op) => encoder.push(op),
            Self::Squircle(ref op) => encoder.push(op),
        }
    }

    fn animate(&mut self) {
        match self {
            Self::Const(ref mut op) => op.animate(),
            Self::Ellipse(ref mut op) => op.animate(),
            Self::Flower(ref mut op) => op.animate(),
            Self::LinearGradient(ref mut op) => op.animate(),
            Self::RadialGradient(ref mut op) => op.animate(),
            Self::PolarTheta(ref mut op) => op.animate(),
            Self::Absolute(ref mut op) => op.animate(),
            Self::Invert(ref mut op) => op.animate(),
            Self::Add(ref mut op) => op.animate(),
            Self::Subtract(ref mut op) => op.animate(),
            Self::Multiply(ref mut op) => op.animate(),
            Self::Divide(ref mut op) => op.animate(),
            Self::Modulus(ref mut op) => op.animate(),
            Self::Exponent(ref mut op) => op.animate(),
            Self::Sinc(ref mut op) => op.animate(),
            Self::Sine(ref mut op) => op.animate(),
            Self::Spiral(ref mut op) => op.animate(),
            Self::Squircle(ref mut op) => op.animate(),
        }
    }
}

#[derive(Debug)]
pub struct Tree {
    layers: [Node; 3],
}

impl Tree {
    pub fn new(rng: &mut StdRng) -> Self {
        Self {
            layers: [
                Node::new(rng, &mut 0, "r"),
                Node::new(rng, &mut 0, "g"),
                Node::new(rng, &mut 0, "b"),
            ],
        }
    }

    pub fn with_layers(r: Node, g: Node, b: Node) -> Self {
        Self { layers: [r, g, b] }
    }

    pub fn show(&self) -> String {
        format!(
            "red:\n{}\ngreen:\n{}\nblue:\n{}\n",
            self.layers[0].show(0),
            self.layers[1].show(0),
            self.layers[2].show(0)
        )
    }

    pub fn animate(&mut self) {
        for layer in self.layers.iter_mut() {
            layer.animate();
        }
    }

    pub fn encode_upload_buffer(
        &self,
        offset: usize,
        device: &wgpu::Device,
    ) -> (wgpu::Buffer, wgpu::Buffer) {
        let mut encoder = InstructionEncoder::new();
        self.layers[offset].encode(&mut encoder);
        let (mut instrs, consts) = encoder.finish();

        let instr_buffer = device
            .create_buffer_mapped(instrs.len(), wgpu::BufferUsage::COPY_SRC)
            .fill_from_slice(&instrs);

        let const_buffer = device
            .create_buffer_mapped(consts.len(), wgpu::BufferUsage::all())
            .fill_from_slice(&consts);

        (instr_buffer, const_buffer)
    }
}
