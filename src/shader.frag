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
    vec2 center = size / 2.0;
    vec4 mixColor;

    vec2 windowSize = size;
    vec2 outlineSize = size + thickness * 2.0;
    vec2 windowCoords = v_coords;
    windowCoords *= windowSize;
    windowCoords -= thickness;
    windowCoords /= windowSize;
    
    vec2 windowLocation = windowCoords * windowSize;
    float windowDistance = rounded_box(windowLocation - center, windowSize / 2.0, radius);
    float smoothedAlpha = 1.0 - smoothstep(0.0, 1.0, abs(windowDistance) - (thickness / 2.0));
    
    if (windowDistance > 0.0) {
        vec2 outlineLocation = v_coords * outlineSize;
        float outlineDistance = rounded_box(windowLocation - center, outlineSize / 2.0, radius);

        if (outlineDistance > 0.0) {
            discard;
        } else {
            mixColor = mix(vec4(0.0, 0.0, 0.0, 0.0), vec4(color, alpha), smoothedAlpha);
        }
    } else {
        vec4 windowColor = texture2D(tex, windowCoords);
        mixColor = mix(windowColor, vec4(color, alpha), smoothedAlpha);
    }
    gl_FragColor = mixColor;
}
