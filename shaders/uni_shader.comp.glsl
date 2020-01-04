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
#version 450
#define PI 3.141592653589793

// In order to facilitate fixed frame rates, we specify a fixed size instruction stream. If the current
// invocation is shorter, it will just get padded with nops.
#define INSTRUCTION_COUNT 128
#define CONSTANT_POOL_SIZE 1024

layout(local_size_x = 8, local_size_y = 8, local_size_z = 1) in;
layout(binding = 0) uniform readonly Configuration {
    ivec2 texture_size;
    ivec2 texture_offsets;
};
layout(binding = 1, r32f) uniform writeonly image2D result_texture;
layout(binding = 2) uniform readonly InstructionStream {
    uint instrs[INSTRUCTION_COUNT];
};
layout(binding = 3) uniform readonly ConstantPool {
    vec4 constant_pool[CONSTANT_POOL_SIZE];
};

float pop_const(inout uint position) {
    float c = constant_pool[position / 4][position % 4];
    position += 1;
    return c;
}

float interpret(vec2 position)
{
    float stack[INSTRUCTION_COUNT * 2];
    uint stack_offset = 0;
    uint coff = 0;
    float size;

    for (int i = 0; i < INSTRUCTION_COUNT; ++i) {
        uint instr = instrs[i];
        uint const_count = (instr >> 16) & 0xFF;
        uint child_count = (instr >> 8) & 0xFF;
        uint op = instr & 0xFF;

        switch(op) {
        case 1: // const
            stack[stack_offset] = pop_const(coff);
            break;
        case 2: // ellipse
            {
                vec2 x0 = vec2(pop_const(coff), pop_const(coff));
                vec2 x1 = vec2(pop_const(coff), pop_const(coff));
                float size = pop_const(coff);
                float sharp = pop_const(coff);
                float dist = distance(position, x0) + distance(position, x1);
                stack[stack_offset] = clamp(size - dist, -1, 1) * sharp;
            }
            break;
        case 3: // flower
            {
                vec2 p = vec2(pop_const(coff), pop_const(coff));
                float angle = pop_const(coff);
                float size = pop_const(coff);
                float ratio = pop_const(coff);
                float n_points = pop_const(coff);
                float sharpness = pop_const(coff);
                vec2 v0 = position - p;
                float d = length(v0);
                vec2 v1 = vec2(
                    v0.x * cos(angle) - v0.y * sin(angle),
                    v0.x * sin(angle) + v0.y * cos(angle)
                );
                float theta = (atan(v1.y, v1.x) / PI + 1.0) / 2.0; // [0,1] on full circle
                float expanded = theta * floor(n_points); // [0,n] on full circle
                float offset = fract(expanded); // [0,1] on each segment
                offset = offset * 2 - 1; // [-1,1] centered on segment
                float inner = size * ratio;
                float r = ((d - inner) * (1.0 / (size - inner))); // ratio from outer to inner
                float dist = r - abs(offset);// - (d / size * ratio * 20);
                stack[stack_offset] = clamp(-dist, -1, 1) * sharpness;
            }
            break;
        case 4: // linear gradient
            {
                vec3 x0 = vec3(pop_const(coff), pop_const(coff), 0);
                vec3 x1 = vec3(pop_const(coff), pop_const(coff), 0);
                float sharpness = pop_const(coff);
                vec3 c = cross(x1 - x0, vec3(position, 0) - x0);
                stack[stack_offset] = smoothstep(-1, 1, c.z * sharpness) * 2 - 1;
            }
            break;
        case 5: // radial gradient
            {
                vec2 x0 = vec2(pop_const(coff), pop_const(coff));
                float w = pop_const(coff);
                float h = pop_const(coff);
                float angle = pop_const(coff);
                vec2 v0 = position - x0;
                vec2 v1 = vec2(
                    v0.x * cos(angle) - v0.y * sin(angle),
                    v0.x * sin(angle) + v0.y * cos(angle)
                );
                vec2 v2 = vec2(v1.x / w, v1.y / h);
                float tmp = -length(v2) * 2 / sqrt(2) + 1;
                stack[stack_offset] = clamp(tmp, -1, 1);
            }
            break;
        case 6: // polar theta
            {
                vec2 x0 = vec2(pop_const(coff), pop_const(coff));
                float angle = pop_const(coff);
                vec2 v0 = position - x0;
                vec2 v1 = vec2(
                    v0.x * cos(angle) - v0.y * sin(angle),
                    v0.x * sin(angle) + v0.y * cos(angle)
                );
                stack[stack_offset] = atan(v1.y, v1.x) / PI;
            }
            break;
        case 8: // absolute
            stack[stack_offset - 1] = abs(stack[stack_offset - 1]);
            break;
        case 9: // invert
            stack[stack_offset - 1] = -stack[stack_offset - 1];
            break;
        case 10: // add
            stack[stack_offset - 2] = stack[stack_offset - 2] + stack[stack_offset - 1];
            break;
        case 11: // sub
            stack[stack_offset - 2] = stack[stack_offset - 2] - stack[stack_offset - 1];
            break;
        case 12: // multiply
            stack[stack_offset - 2] = stack[stack_offset - 2] * stack[stack_offset - 1];
            break;
        case 13: // divide
            stack[stack_offset - 2] = stack[stack_offset - 2] / stack[stack_offset - 1];
            break;
        case 14: // modulus
            stack[stack_offset - 2] = mod(stack[stack_offset - 2], stack[stack_offset - 1]);
            break;
        case 15: // exponentiate
            stack[stack_offset - 2] = pow(stack[stack_offset - 2], stack[stack_offset - 1]);
            break;
        case 16: // sinc
            {
                float freq = pop_const(coff);
                float phase = pop_const(coff);
                float denom = stack[stack_offset - 1] * freq + phase;
                stack[stack_offset - 1] = clamp(sin(denom) / denom, -1, 1);
            }
            break;
        case 17: // sine
            {
                float freq = pop_const(coff);
                float phase = pop_const(coff);
                stack[stack_offset - 1] = sin(stack[stack_offset - 1] * freq + phase);
            }
            break;
        case 18: // spiral
            {
                vec2 center = vec2(pop_const(coff), pop_const(coff));
                float n = pop_const(coff);
                float b = pop_const(coff);
                vec2 v0 = position - center;

                float r = (v0.x * v0.x + v0.y * v0.y) * 2 / sqrt(2) - 1;
                float theta = atan(v0.y, v0.x) / PI;
                float tmp = abs(abs(stack[stack_offset - 1]) - 0.5);
                stack[stack_offset - 1] = 4 * tmp - 1;
            }
            break;
        case 19: // squircle
            {
                vec2 x0 = vec2(pop_const(coff), pop_const(coff));
                float r = pop_const(coff);
                float n = pop_const(coff);
                vec2 v0 = position - x0;
                float a = abs(v0.x - stack[stack_offset - 2]);
                float b = abs(v0.y - stack[stack_offset - 1]);
                float numer = -(pow(a, n) + pow(b, n));
                float denom = pow(r, n);
                stack[stack_offset - 2] = clamp(numer / denom, -1, 1);
            }
            break;
        default:
            continue;
        }

        stack_offset -= (child_count - 1);
    }

    return stack[0];
}

void main()
{
    // All computations are done as if on a square. We render to a screen-sized
    // slice of that square and just skip over pixels that are out of the screen area.
    ivec2 pixel_index = ivec2(gl_GlobalInvocationID.xy);
    vec2 position = vec2(
        (float(pixel_index.x + texture_offsets.x) / float(texture_size.x)) * 2.0 - 1.0,
        (float(pixel_index.y + texture_offsets.y) / float(texture_size.x)) * 2.0 - 1.0
    );

    float result = (interpret(position) + 1.0) / 2.0;

    imageStore(result_texture, pixel_index, vec4(result, 0, 0, 0));
}
