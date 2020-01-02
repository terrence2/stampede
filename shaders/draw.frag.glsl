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

layout(location = 0) in vec2 v_tex_coord;

layout(location = 0) out vec4 f_color;

layout(binding = 0) uniform texture2D result_texture;
layout(binding = 1) uniform sampler result_sampler;

void main() {
    f_color = texture(sampler2D(result_texture, result_sampler), v_tex_coord);
}
