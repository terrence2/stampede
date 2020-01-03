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
use rand::prelude::*;
use std::mem;
use wgpu;

const INSTRUCTION_COUNT: usize = 512;
const CONSTANT_POOL_SIZE: usize = 1024;

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

    pub fn finish(mut self) -> ([u32; INSTRUCTION_COUNT], [f32; CONSTANT_POOL_SIZE]) {
        // Fill end with nops
        while self.instr_offset % INSTRUCTION_COUNT != 0 {
            self.push_2(0, None, None);
        }

        (self.instrs, self.constant_pool)
    }

    fn encode_arg(a: Option<u16>) -> (u32, u32) {
        if let Some(v) = a {
            (1, v as u32)
        } else {
            (0, 0)
        }
    }

    pub fn push_1(&mut self, op: u8, arg0: Option<u16>) {
        self.push_2(op, arg0, None);
    }

    // One word encoding. Identical first word to the encoding of the second word.
    // Yes, we have to know the semantics of the opcode to know the number of words to decode.
    // oooo_ooFF 0000_0000 0000_1111 1111_1111
    pub fn push_2(&mut self, op: u8, arg0: Option<u16>, arg1: Option<u16>) {
        assert_eq!(op & 0b1100_0000, 0);
        let (a0f, a0) = Self::encode_arg(arg0);
        let (a1f, a1) = Self::encode_arg(arg1);
        let instr0 = (op as u32) << 26 |
            a0f << 25 |
            a1f << 24 |
            (a0 & 0x0FFF) << 12 |
            (a1 & 0x0FFF);
        self.instrs[self.instr_offset] = instr0;
    }

    /*
    // Encoding is 8 bits for the opcode, then 14 bits for each of 4 args.
    // oooo_ooFF 1111_1111 1111_2222 2222_2222
    // XXXXXXXFF 3333_3333 3333_4444 4444_4444
    pub fn push_4(&mut self, op: u8, arg0: Option<u16>, arg1: Option<u16>, arg2: Option<u16>, arg3: Option<u16>) {
        self.push_2(op, arg0, arg1);
        self.push_2(0, arg2, arg3);
    }
    */

    pub fn push_constant(&mut self, value: f32) -> u16 {
        let out = self.pool_offset as u16;
        self.constant_pool[self.pool_offset] = value;
        self.pool_offset += 1;
        out
    }
}

fn prefix(level: usize) -> String {
    let mut s = String::new();
    for _ in 0..level {
        s += " ";
    }
    s
}

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

#[derive(Debug)]
pub struct AddOp {
    lhs: Box<Node>,
    rhs: Box<Node>,
}

impl AddOp {
    fn new(rng: &mut ThreadRng, count: &mut usize) -> Self {
        Self {
            lhs: Box::new(Node::new(rng, count)),
            rhs: Box::new(Node::new(rng, count)),
        }
    }

    pub fn with_children(lhs: Node, rhs: Node) -> Self {
        Self {
            lhs: Box::new(lhs),
            rhs: Box::new(rhs),
        }
    }

    fn show(&self, level: usize) -> String {
        let p = prefix(level);
        format!(
            "{}Add-\n{}\n{}",
            p,
            self.lhs.show(level + 1),
            self.rhs.show(level + 1)
        )
    }

    fn encode(&self, opcode: u8, encoder: &mut InstructionEncoder) -> Option<u16> {
        let c0 = self.lhs.encode(encoder);
        let c1 = self.rhs.encode(encoder);
        encoder.push_2(opcode, c0, c1);
        None
    }

}

/*
pub enum OpNode {
    // Unary
    //    Absolute,
    //    Invert,

    // Binary
    //    Divide,
    //    Exponentiate,
    //    Modulus,
    //    Multiply,
    //
    //    // Trig
    //    Sinc,
    //    Sine,
    //    Spiral,
    //    Squircle,
}
pub enum LeafNode {
    Const(ConstOp),
    //    Ellipse,
    //    Flower,
    //    LinearGradient,
    //    RadialGradient,
    //    PolarTheta
}
*/

#[derive(Debug)]
pub enum Node {
    // Leaves
    Const(f32),

    // Operations
    Add(AddOp),
}

impl Node {
    fn new(rng: &mut ThreadRng, count: &mut usize) -> Self {
        // FIXME: pick a better walk for this
        //let fullness = *count as f32 / INSTRUCTION_COUNT as f32;
        let fullness = 1f32;
        *count += 1;
        if rng.gen_range(0f32, 1f32) < fullness {
            // Leaf distribution
            println!("CAPPING: {}", fullness);
            Self::Const(rng.gen_range(0f32, 1f32))
        } else {
            // Operation distribution
            println!("EXPAND: {}", fullness);
            Self::Add(AddOp::new(rng, count))
        }
    }

    fn show(&self, level: usize) -> String {
        match self {
            Self::Const(ref value) => format!("{:0.2}", value),
            Self::Add(ref add_op) => add_op.show(level + 1),
        }
    }

    fn encode(&self, encoder: &mut InstructionEncoder) -> Option<u16> {
        match self {
            Self::Const(ref value) => Some(encoder.push_constant(*value)),
            Self::Add(ref add_op) => {
                add_op.encode(0x12, encoder);
                None
            }
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
                Node::new(rng, &mut 0),
                Node::new(rng, &mut 0),
                Node::new(rng, &mut 0),
            ],
        }
    }

    pub fn with_layers(r: Node, g: Node, b: Node) -> Self {
        Self {
            layers: [r, g, b]
        }
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
        if let Some(const_ref) = self.layers[offset].encode(&mut encoder) {
            encoder.push_1(1, Some(const_ref));
        }
        let (instrs, consts) = encoder.finish();
//        println!("instrs: {:X}, {}", instrs[0], instrs[1]);
//        println!("consts: {}, {}", consts[0], consts[1]);

        let instr_buffer = device
            .create_buffer_mapped(instrs.len(), wgpu::BufferUsage::COPY_SRC)
            .fill_from_slice(&instrs);

        let const_buffer = device
            .create_buffer_mapped(consts.len(), wgpu::BufferUsage::COPY_SRC)
            .fill_from_slice(&consts);

        (instr_buffer, const_buffer)
    }
}
