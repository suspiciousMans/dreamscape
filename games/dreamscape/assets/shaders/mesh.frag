#ifdef AFFINE_UV
noperspective in vec2 vUV;
#else
in vec2 vUV;
#endif
in vec3 vLight;
in float vFogFactor;
in vec3 vWorld;

out vec4 FragColor;

uniform sampler2D uTex;
uniform vec3 uFogColor;
uniform float uTime;
uniform float uStrangeness;
uniform float uMelt;

// Rotate a colour around the grey axis (hue shift that keeps brightness).
vec3 hueShift(vec3 c, float a) {
    const vec3 k = vec3(0.57735);
    float ca = cos(a);
    return c * ca + cross(k, c) * sin(a) + k * dot(k, c) * (1.0 - ca);
}

void main() {
    float s = uStrangeness;
    // Melting UVs: the pattern drifts and ripples across the surface.
    vec2 uv = vUV;
    uv += s * 0.08 * vec2(sin(uTime * 0.7 + vUV.y * 6.2832), cos(uTime * 0.5 + vUV.x * 6.2832));
    uv += s * 0.05 * uTime * vec2(0.3, 0.2);

    vec4 texColor = texture(uTex, uv);

    // Rainbow waves rolling through the world; faster and wider the deeper you go.
    float wave = uTime * 0.4 + dot(vWorld, vec3(0.06, 0.12, 0.05));
    vec3 col = hueShift(texColor.rgb, s * 2.2 * sin(wave));
    // A little saturation pump at high strangeness.
    float grey = dot(col, vec3(0.299, 0.587, 0.114));
    col = clamp(mix(vec3(grey), col, 1.0 + 0.5 * s), 0.0, 1.0);

    vec3 lit = col * vLight;
    // Fog cycles hue too, so the horizon itself shimmers.
    vec3 fog = hueShift(uFogColor, s * 0.8 * sin(uTime * 0.25));
    vec3 withFog = mix(lit, fog, vFogFactor);
    // Melting: colours smear round the hue wheel, then everything dissolves
    // into the fog so the dream swap underneath is never seen.
    withFog = mix(withFog, hueShift(withFog, 4.0 * uMelt), uMelt);
    withFog = mix(withFog, fog, smoothstep(0.45, 0.95, uMelt));
    FragColor = vec4(withFog, texColor.a);
}
