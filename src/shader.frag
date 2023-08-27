precision mediump float;
uniform float alpha;
uniform vec2 size;
varying vec2 v_coords;

uniform vec3 color;
uniform float thickness;
uniform float radius;

float rounded_box(vec2 center, vec2 size, float radius) {
    return length(max(abs(center) - size + radius, 0.0)) - radius;
}

void main() {
    vec2 center = size / 2.0;
    vec2 location = v_coords * size;
    vec4 mix_color;

    float distance = rounded_box(location - center, (size / 2.0) - (thickness / 2.0), radius);
    float smoothedAlpha = 1.0 - smoothstep(0.0, 1.0, abs(distance) - (thickness / 2.0));
    
    mix_color = mix(vec4(0.0, 0.0, 0.0, 0.0), vec4(color, alpha), smoothedAlpha);
    
    gl_FragColor = mix_color;
}
