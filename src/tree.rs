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

fn prefix(level: usize) -> String {
    let mut s = String::new();
    for _ in 0..level {
        s += " ";
    }
    s
}

#[derive(Debug)]
pub struct ConstOp(u8);
impl ConstOp {
    fn new(_level: usize, rng: &mut ThreadRng) -> Self {
        Self(rng.gen::<u8>())
    }

    fn show(&self, level: usize) -> String {
        format!("{}Const({})", prefix(level), self.0)
    }
}

#[derive(Debug)]
pub enum LeafNode {
    Const(ConstOp),
    //    Ellipse,
    //    Flower,
    //    LinearGradient,
    //    RadialGradient,
    //    PolarTheta
}

impl LeafNode {
    fn new(level: usize, rng: &mut ThreadRng) -> Self {
        Self::Const(ConstOp::new(level + 1, rng))
    }

    fn show(&self, level: usize) -> String {
        match self {
            Self::Const(op) => op.show(level),
        }
    }
}

#[derive(Debug)]
pub struct AddOp {
    lhs: Box<Node>,
    rhs: Box<Node>,
}

impl AddOp {
    fn new(level: usize, rng: &mut ThreadRng) -> Self {
        Self {
            lhs: Box::new(Node::new(level + 1, rng)),
            rhs: Box::new(Node::new(level + 1, rng)),
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
}

#[derive(Debug)]
pub enum OpNode {
    // Unary
    //    Absolute,
    //    Invert,

    // Binary
    Add(AddOp),
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

impl OpNode {
    fn new(level: usize, rng: &mut ThreadRng) -> Self {
        Self::Add(AddOp::new(level, rng))
    }

    fn show(&self, level: usize) -> String {
        match self {
            Self::Add(ref op) => op.show(level),
        }
    }
}

#[derive(Debug)]
pub enum Node {
    Leaf(LeafNode),
    Operation(OpNode),
}

impl Node {
    fn new(level: usize, rng: &mut ThreadRng) -> Self {
        // FIXME: pick a better walk for this
        if rng.gen::<bool>() {
            Node::Operation(OpNode::new(level + 1, rng))
        } else {
            Node::Leaf(LeafNode::new(level + 1, rng))
        }
    }

    fn show(&self, level: usize) -> String {
        match self {
            Self::Leaf(ref node) => node.show(level + 1),
            Self::Operation(ref node) => node.show(level + 1),
        }
    }
}

#[derive(Debug)]
pub struct Tree {
    r: Node,
    g: Node,
    b: Node,
}

impl Tree {
    pub fn new(rng: &mut ThreadRng) -> Self {
        Self {
            r: Node::new(0, rng),
            g: Node::new(0, rng),
            b: Node::new(0, rng),
        }
    }

    pub fn show(&self) -> String {
        format!(
            "red:\n{}\ngreen:\n{}\nblue:\n{}\n",
            self.r.show(0),
            self.g.show(0),
            self.b.show(0)
        )
    }
}
