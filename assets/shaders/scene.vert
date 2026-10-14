#version 100
attribute vec3 position;
attribute vec3 color;
uniform mat4 vp;
varying lowp vec3 vertex_color;
void main() {
    gl_Position = vp * vec4(position, 1.0);
    vertex_color = color;
}
