// This file is part of Arctic.
//
// Arctic is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// Arctic is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with Arctic.  If not, see <http://www.gnu.org/licenses/>.
use rand::{prelude::*, distributions::Uniform};
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
        for &v in consts {
            self.push_constant(v);
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
    fn get_constants(&self) -> &[f32];
    fn get_children(&self) -> &[Box<Node>];
}

fn prefix(level: usize) -> String {
    let mut s = String::new();
    for _ in 0..level {
        s += " ";
    }
    s
}

/*
#[derive(Copy, Clone)]
struct State {
    count: usize,
}

impl State {
    fn descend(mut self) -> Self {
        self.count += 1;
        self
    }
}
*/

macro_rules! make_op {
    ($op_name:ident [$opcode:literal] {
        constants($const_count:literal) => [$($const_name:ident[$min_bound:expr,$max_bound:expr]),*],
        children($child_count:literal) => [$($child_name:ident),*]
    }) => {
        #[derive(Debug)]
        pub struct $op_name {
            consts: [f32; $const_count],
            children: [Box<Node>; $child_count]
        }

        impl $op_name {
            pub fn new(rng: &mut ThreadRng, _count: &mut usize) -> Self {
                Self {
                    consts: [
                        $(
                            rng.gen_range(($min_bound) as f32, ($max_bound) as f32)
                        ),*
                    ],
                    children: [
                        $(
                            Box::new(Node::new(rng, _count, stringify!($child_name)))
                        ),*
                    ],
                }
            }

            #[allow(dead_code)]
            pub fn with_constants($($const_name: f32),*) -> Self {
                let _rng = &mut thread_rng();
                let _count = &mut 0;
                Self {
                    consts: [
                        $($const_name),*
                    ],
                    children: [
                        $(
                            Box::new(Node::new(_rng, _count, stringify!($child_name)))
                        ),*
                    ],
                }
            }

            pub fn show(&self, level: usize) -> String {
                let cc = self.consts.iter().map(|v| format!("{:0.2}", v)).collect::<Vec<String>>().join(", ");
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

            fn get_constants(&self) -> &[f32] {
                &self.consts
            }

            fn get_children(&self) -> &[Box<Node>] {
                &self.children
            }
        }
    }
}

make_op!(ConstOp          [1] { constants(1) => [ value[-1,1] ], children(0) => [] });
make_op!(EllipseOp        [2] { constants(6) => [ f0x[-1,1], f0y[-1,1], f1x[-1,1], f1y[-1,1], size[0.1,1], sharp[1,100] ], children(0) => [] });
make_op!(FlowerOp         [3] { constants(7) => [ center_x[-1,1], center_y[-1,1], angle[0,2.0*PI], size[0,2.5], ratio[0,1], n_points[3,25], sharpness[2,10] ], children(0) => [] });
make_op!(LinearGradientOp [4] { constants(5) => [ p0x[-1,1], p0y[-1,1], p1x[-1,1], p1y[-1,1], sharp[2,20] ], children(0) => [] });
make_op!(RadialGradientOp [5] { constants(5) => [ p0x[-1,1], p0y[-1,1], p1x[-1,1], p1y[-1,1], angle[0,2.0*PI] ], children(0) => [] });
make_op!(PolarThetaOp     [6] { constants(3) => [ x[-1,1], y[-1,1], angle[0,2.0*PI] ], children(0) => [] });
//
//
make_op!(AbsoluteOp       [8] { constants(0) => [], children(1) => [value] });
make_op!(InvertOp         [9] { constants(0) => [], children(1) => [value] });
make_op!(AddOp           [10] { constants(0) => [], children(2) => [lhs, rhs] });
make_op!(SubtractOp      [11] { constants(0) => [], children(2) => [lhs, rhs] });
make_op!(MultiplyOp      [12] { constants(0) => [], children(2) => [lhs, rhs] });
make_op!(DivideOp        [13] { constants(0) => [], children(2) => [lhs, rhs] });
make_op!(ModulusOp       [14] { constants(0) => [], children(2) => [lhs, rhs] });
make_op!(ExponentOp      [15] { constants(0) => [], children(2) => [lhs, rhs] });

/*
pub enum OpNode {
    // Binary
    //    // Trig
    //    Sinc,
    //    Sine,
    //    Spiral,
    //    Squircle,
}
pub enum LeafNode {
    //    PolarTheta
}
*/

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
}

impl Node {
    fn new(rng: &mut ThreadRng, count: &mut usize, _link_name: &str) -> Self {
        // FIXME: pick a better walk for this
        let fullness = (*count * 2) as f32 / INSTRUCTION_COUNT as f32;
        *count += 1;
        if rng.gen_range(0f32, 1f32) < fullness {
            let x = rng.sample(Uniform::new(EllipseOp::opcode(), PolarThetaOp::opcode() + 1));
            match x {
                2 => Self::Ellipse(EllipseOp::new(rng, count)),
                3 => Self::Flower(FlowerOp::new(rng, count)),
                4 => Self::LinearGradient(LinearGradientOp::new(rng, count)),
                5 => Self::RadialGradient(RadialGradientOp::new(rng, count)),
                6 => Self::PolarTheta(PolarThetaOp::new(rng, count)),
                _ => panic!("unknown const opcode")
            }
        } else {
            let x = rng.sample(Uniform::new(AbsoluteOp::opcode(), ExponentOp::opcode() + 1));
            match x {
                8 => Self::Absolute(AbsoluteOp::new(rng, count)),
                9 => Self::Invert(InvertOp::new(rng, count)),
                10 => Self::Add(AddOp::new(rng, count)),
                11 => Self::Subtract(SubtractOp::new(rng, count)),
                12 => Self::Multiply(MultiplyOp::new(rng, count)),
                13 => Self::Divide(DivideOp::new(rng, count)),
                14 => Self::Modulus(ModulusOp::new(rng, count)),
                15 => Self::Exponent(ExponentOp::new(rng, count)),
                _ => panic!("unknown opcode")
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
        }
    }
}

#[derive(Debug)]
pub struct Tree {
    layers: [Node; 3],
}

impl Tree {
    pub fn new(rng: &mut ThreadRng) -> Self {
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

    pub fn encode_upload_buffer(
        &self,
        offset: usize,
        device: &wgpu::Device,
    ) -> (wgpu::Buffer, wgpu::Buffer) {
        let mut encoder = InstructionEncoder::new();
        self.layers[offset].encode(&mut encoder);
        let (instrs, consts) = encoder.finish();
//        println!(
//            "instrs: {:06X}, {:06X}, {:06X}",
//            instrs[0], instrs[1], instrs[2]
//        );
//        println!("consts: {:?}", &consts[..10]);

        let instr_buffer = device
            .create_buffer_mapped(instrs.len(), wgpu::BufferUsage::COPY_SRC)
            .fill_from_slice(&instrs);

        let const_buffer = device
            .create_buffer_mapped(consts.len(), wgpu::BufferUsage::all())
            .fill_from_slice(&consts);

        (instr_buffer, const_buffer)
    }
}
