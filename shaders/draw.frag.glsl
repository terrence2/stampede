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

vec3 lab2xyz( vec3 c ) {
    float fy = ( c.x + 16.0 ) / 116.0;
    float fx = c.y / 500.0 + fy;
    float fz = fy - c.z / 200.0;
    return vec3(
         95.047 * (( fx > 0.206897 ) ? fx * fx * fx : ( fx - 16.0 / 116.0 ) / 7.787),
        100.000 * (( fy > 0.206897 ) ? fy * fy * fy : ( fy - 16.0 / 116.0 ) / 7.787),
        108.883 * (( fz > 0.206897 ) ? fz * fz * fz : ( fz - 16.0 / 116.0 ) / 7.787)
    );
}

vec3 xyz2rgb( vec3 c ) {
	const mat3 mat = mat3(
        3.2406, -1.5372, -0.4986,
        -0.9689, 1.8758, 0.0415,
        0.0557, -0.2040, 1.0570
	);
    vec3 v = mat * (c / 100.0);
    vec3 r;
    r.x = ( v.r > 0.0031308 ) ? (( 1.055 * pow( v.r, ( 1.0 / 2.4 ))) - 0.055 ) : 12.92 * v.r;
    r.y = ( v.g > 0.0031308 ) ? (( 1.055 * pow( v.g, ( 1.0 / 2.4 ))) - 0.055 ) : 12.92 * v.g;
    r.z = ( v.b > 0.0031308 ) ? (( 1.055 * pow( v.b, ( 1.0 / 2.4 ))) - 0.055 ) : 12.92 * v.b;
    return r;
}

vec3 lab2rgb( vec3 c ) {
    return xyz2rgb( lab2xyz( vec3(100.0 * c.x, 2.0 * 127.0 * (c.y - 0.5), 2.0 * 127.0 * (c.z - 0.5)) ) );
}

void main() {
    // Project into RGB from a more linear color space to avoid causing (extra) non-uniform color shifts.
    float l = 100 * texture(sampler2D(r_texture, r_sampler), v_tex_coord).r;
    float a = (255 * texture(sampler2D(g_texture, g_sampler), v_tex_coord).r) - 128;
    float b = (255 * texture(sampler2D(b_texture, b_sampler), v_tex_coord).r) - 128;
    f_color = vec4(lab2rgb(vec3(l, a, b)), 1);
}
