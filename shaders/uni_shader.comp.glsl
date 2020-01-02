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

// All computations are done as if on a square. We render to a screen-sized
// slice of that square and just skip over pixels that are out of the screen area.
#define RESULT_SIZE 1920
#define TEXTURE_WIDTH 1920
#define TEXTURE_HEIGHT 1080
#define TEXTURE_Y_OFFSET 420 /* (1920 - 1080) / 2 */

// In order to facilitate fixed frame rates, we specify a fixed size instruction stream. If the current
// invocation is shorter, it will just get padded with nops.
#define INSTRUCTION_COUNT 1024

layout(local_size_x = 8, local_size_y = 8, local_size_z = 1) in;
layout(binding = 0, rgba32f) uniform writeonly image2D result_texture;
layout(binding = 1) uniform readonly InstructionStream {
    uint instrs[INSTRUCTION_COUNT];
};

void main()
{
    ivec2 pixel_index = ivec2(gl_GlobalInvocationID.xy);
    vec2 position = vec2(pixel_index.x / float(TEXTURE_WIDTH), (pixel_index.y + TEXTURE_Y_OFFSET) / float(TEXTURE_WIDTH));

    vec4 stack = vec4(0);
    float cnt = 0;
    for (int i = 0; i < INSTRUCTION_COUNT; ++i) {
        if (instrs[i] == 0) {
            ; // pass
        } else {
            cnt += 1.0;
        }
    }

    float result = cnt / float(INSTRUCTION_COUNT);

    imageStore(result_texture, pixel_index, vec4(result, result, result, 1));
}
