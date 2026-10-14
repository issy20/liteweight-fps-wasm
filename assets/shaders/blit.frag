#version 100
uniform sampler2D scene;
varying mediump vec2 uv;
void main() { gl_FragColor = texture2D(scene, uv); }
