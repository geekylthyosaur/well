precision mediump float;
varying vec2 v_coords;
uniform float alpha;
uniform vec2 size;

uniform vec3 color;
uniform float thickness;
uniform float radius;
uniform sampler2D tex;

float rounded_box(vec2 center, vec2 size, float radius) {
    return length(max(abs(center) - size + radius, 0.0)) - radius;
}

void main() {
    vec2 center = size / 2.0;
    vec2 location = v_coords * size;

    float distance = rounded_box(location - center, (size / 2.0) - (thickness / 2.0), radius);
    float smoothedAlpha = 1.0 - smoothstep(0.0, 1.0, abs(distance) - (thickness / 2.0));
    
    if (distance > 0.0) { // Discard pixels outside the rounded rectangle
        discard;
    }

    vec4 windowColor = texture2D(tex, v_coords);
    vec4 mixColor = mix(windowColor, vec4(color, alpha), smoothedAlpha);
    
    gl_FragColor = windowColor;
}
