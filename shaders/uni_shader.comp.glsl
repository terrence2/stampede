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
#version 450

// In order to facilitate fixed frame rates, we specify a fixed size instruction stream. If the current
// invocation is shorter, it will just get padded with nops.
#define INSTRUCTION_COUNT 512
#define CONSTANT_POOL_SIZE 1024

layout(local_size_x = 8, local_size_y = 8, local_size_z = 1) in;
layout(binding = 2) uniform readonly Configuration {
    ivec2 texture_size;
    ivec2 texture_offsets;
};
layout(binding = 1, rgba32f) uniform writeonly image2D result_texture;
layout(binding = 2) uniform readonly InstructionStream {
    uint instrs[INSTRUCTION_COUNT];
};
layout(binding = 3) uniform readonly ConstantPool {
    float constant_pool[CONSTANT_POOL_SIZE];
};

float interpret()
{
    float stack[128];
    uint stack_offset = 0;

    for (int i = 0; i < INSTRUCTION_COUNT; ++i) {
        uint instr = instrs[i];
        uint op = instr >> 26;
        uint a0f = (instr >> 25) & 1;
        uint a1f = (instr >> 24) & 1;
        uint arg0 = (instr >> 12) & 0xFFF;
        uint arg1 = (instr >> 0) & 0xFFF;

        switch(op) {
        case 1: // const
            stack[stack_offset] = constant_pool[arg0];
            stack_offset += 1;
        case 0x12: // add
            float lhs = 1.0;
            if (a0f == 1) {
                lhs = constant_pool[arg0];
            } else {
                stack_offset -= 1;
                lhs = stack[stack_offset];
            }
            float rhs = 1.0;
            if (a1f == 1) {
                rhs = constant_pool[arg1];
            } else {
                stack_offset -= 1;
                rhs = stack[stack_offset];
            }
            stack[stack_offset] = (lhs + rhs) / 2.0;
            stack_offset += 1;
        default:
            continue;
        }
    }

    return stack[0];
}

void main()
{
    // All computations are done as if on a square. We render to a screen-sized
    // slice of that square and just skip over pixels that are out of the screen area.
    ivec2 pixel_index = ivec2(gl_GlobalInvocationID.xy);
    vec2 position = vec2(
        (pixel_index.x + texture_offsets.x) / float(texture_size.x),
        (pixel_index.y + texture_offsets.y) / float(texture_size.x)
    );

    float result = interpret();

    imageStore(result_texture, pixel_index, vec4(result, 0, 0, 0));
}
