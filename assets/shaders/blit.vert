#version 100
attribute vec2 position;
varying mediump vec2 uv;
void main() { gl_Position = vec4(position, 0., 1.); uv = position * 0.5 + 0.5; }
