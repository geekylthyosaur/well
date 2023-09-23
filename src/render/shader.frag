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

#if defined(DEBUG_FLAGS)
uniform float tint;
#endif

float box(vec2 center, vec2 size, float radius) {
    vec2 dist = abs(center) - size;
    if (radius - thickness > 0.0) {
        return length(max(dist + radius, 0.0)) - radius;
    } else {
        return max(dist.x, dist.y);
    }
}

void main() {
    vec4 mixColor;
    vec4 outlineColor = thickness == 0.0 ? vec4(0) : vec4(color, alpha);

    vec2 windowSize = size - thickness * 2.0;
    vec2 windowCoords = v_coords - thickness / windowSize;
    float windowDistance = box(windowSize * (windowCoords - 0.5), windowSize / 2.0 + thickness, radius + thickness);

    if (windowDistance > 0.0) {
        float smoothedAlpha = 1.0 - smoothstep(0.0, 1.0, windowDistance);
        mixColor = mix(vec4(0), outlineColor, smoothedAlpha);
    } else {
        vec4 windowColor = texture2D(tex, windowCoords);
        float smoothedAlpha = 1.0 - smoothstep(0.0, 1.0, abs(windowDistance) - thickness);
        mixColor = mix(windowColor, outlineColor, smoothedAlpha);
    }

    #if defined(NO_ALPHA)
        mixColor = vec4(mixColor.rgb, 1.0) * alpha;
    #else
        mixColor *= alpha;
    #endif

    #if defined(DEBUG_FLAGS)
        if (tint == 1.0)
            mixColor = vec4(0.0, 0.3, 0.0, 0.2) + mixColor * 0.8;
    #endif

    gl_FragColor = mixColor;
}
