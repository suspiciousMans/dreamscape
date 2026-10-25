#version 330 core
in vec2 vUV;
out vec4 FragColor;

uniform sampler2D uSourceTex;
uniform float uColorLevels;
uniform float uDitherStrength;
uniform vec3 uTintColor;
uniform float uTintStrength;

// Values 0..15 arranged for a reasonably well-spread ordered-dither pattern.
const mat4 kBayer = mat4(
     0.0,  8.0,  2.0, 10.0,
    12.0,  4.0, 14.0,  6.0,
     3.0, 11.0,  1.0,  9.0,
    15.0,  7.0, 13.0,  5.0
) / 16.0;

// Vignette + film grain, laid over the finished frame (what the egui overlay
// used to draw as two full-screen blended layers). Off unless uOvOn is set.
uniform int uOvOn;
uniform vec2 uOvCentre;     // vignette centre, window pixels, origin top-left
uniform vec3 uOvRadii;      // half diagonal (px), inner, outer (fractions)
uniform float uOvRing[11];  // darkness at inner + (outer - inner) * i / 10
uniform vec4 uOvGrain;      // tile offset (uv), tile size (px), strength
uniform sampler2D uOvGrainTex;
uniform float uOvViewH;

vec3 overlay(vec3 c) {
    vec2 p = vec2(gl_FragCoord.x, uOvViewH - gl_FragCoord.y);
    float d = length(p - uOvCentre) / uOvRadii.x;
    float t = clamp((d - uOvRadii.y) / max(uOvRadii.z - uOvRadii.y, 1e-4), 0.0, 1.0) * 10.0;
    int i = min(int(floor(t)), 9);
    float a = mix(uOvRing[i], uOvRing[i + 1], t - float(i));
    c *= 1.0 - a;
    if (uOvGrain.w > 0.0) {
        vec4 g = texture(uOvGrainTex, uOvGrain.xy + p / uOvGrain.z) * uOvGrain.w;
        c = c * (1.0 - g.a) + g.rgb;
    }
    return c;
}

void main() {
    vec4 color = texture(uSourceTex, vUV);

    ivec2 cell = ivec2(mod(gl_FragCoord.xy, 4.0));
    float threshold = kBayer[cell.x][cell.y] - 0.5;

    vec3 tinted = mix(color.rgb, uTintColor, uTintStrength);

    float levels = max(uColorLevels, 1.0);
    vec3 dithered = tinted + threshold * uDitherStrength / levels;
    vec3 quantized = floor(dithered * levels + 0.5) / levels;

    vec3 outc = clamp(quantized, 0.0, 1.0);
    if (uOvOn != 0) {
        outc = clamp(overlay(outc), 0.0, 1.0);
    }
    FragColor = vec4(outc, color.a);
}
