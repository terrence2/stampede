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

layout(location = 0) in vec2 v_tex_coord;

layout(location = 0) out vec4 f_color;

layout(binding = 0) uniform texture2D r_texture;
layout(binding = 1) uniform sampler r_sampler;
layout(binding = 2) uniform texture2D g_texture;
layout(binding = 3) uniform sampler g_sampler;
layout(binding = 4) uniform texture2D b_texture;
layout(binding = 5) uniform sampler b_sampler;

void main() {
    f_color = vec4(
        texture(sampler2D(r_texture, r_sampler), v_tex_coord).r,
        texture(sampler2D(g_texture, g_sampler), v_tex_coord).r,
        texture(sampler2D(b_texture, b_sampler), v_tex_coord).r,
        1.0
    );
}
