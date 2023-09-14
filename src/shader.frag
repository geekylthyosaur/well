#if defined(EXTERNAL)
#extension GL_OES_EGL_image_external : require
#endif

precision mediump float;
#if defined(EXTERNAL)
uniform samplerExternalOES tex;
#else
uniform sampler2D tex;
#endif
uniform float alpha;
varying vec2 v_coords;

uniform vec3 color;
uniform float thickness;
uniform float radius;
uniform vec2 size;

float rounded_box(vec2 center, vec2 size, float radius) {
    return length(max(abs(center) - size + radius, 0.0)) - radius;
}

void main() {
    // Border is not supported yet
    float thickness = 0.0;
    vec2 center = size / 2.0;
    vec2 location = v_coords * size;

    float distance = rounded_box(location - center, (size / 2.0) - (thickness / 2.0), radius);
    float smoothedAlpha = 1.0 - smoothstep(0.0, 1.0, abs(distance) - (thickness / 2.0));
    
    // Discard pixels outside the rounded rectangle
    if (distance > 0.0) {
        discard;
    }

    vec4 windowColor = texture2D(tex, v_coords);
    vec4 mixColor = mix(windowColor, vec4(color, alpha), smoothedAlpha);
    
    gl_FragColor = mixColor;
}
