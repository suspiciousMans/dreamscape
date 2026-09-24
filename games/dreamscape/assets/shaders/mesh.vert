layout (location = 0) in vec3 aPos;
layout (location = 1) in vec3 aNormal;
layout (location = 2) in vec2 aUV;
layout (location = 3) in vec3 aColor;

uniform mat4 uModel;
uniform mat4 uView;
uniform mat4 uProj;
uniform vec3 uLightDir;
uniform vec3 uAmbientColor;
uniform int uLightingMode;
uniform float uVertexSnapAmount;
uniform float uFogStart;
uniform float uFogEnd;

// Dreamscape: acid-trip controls.
uniform float uTime;
// 0 = sober, 1 = fully melted. Scales every distortion below.
uniform float uStrangeness;
// > 0: texture coordinates come from world position (textures tile at this
// many repeats per world unit instead of stretching over big slabs).
uniform float uUVScale;
// Dream transition: 0 = solid, 1 = fully melted into the floor.
uniform float uMelt;

// Fixed-size point light arrays (simple uniform arrays, not a UBO/SSBO —
// plenty for a handful of level lights and keeps the shader trivial).
uniform vec3 uPointLightPos[4];
uniform vec3 uPointLightColor[4];
uniform float uPointLightIntensity[4];
uniform float uPointLightRange[4];
uniform int uPointLightCount;

#ifdef AFFINE_UV
noperspective out vec2 vUV;
#else
out vec2 vUV;
#endif
out vec3 vLight;
out float vFogFactor;
out vec3 vWorld;

void main() {
    vec4 worldPos = uModel * vec4(aPos, 1.0);
    vec3 worldNormal = normalize(mat3(uModel) * aNormal);

    // World-space (tri-planar style) UVs, from the un-warped position so the
    // pattern stays glued to the surface while the geometry breathes.
    if (uUVScale > 0.0) {
        vec3 n = abs(worldNormal);
        vec2 uv = n.y > 0.5 ? worldPos.xz : (n.x > 0.5 ? worldPos.zy : worldPos.xy);
        vUV = uv * uUVScale;
    } else {
        vUV = aUV;
    }
    vWorld = worldPos.xyz;

    // The world breathes: slow travelling waves through every vertex.
    float breathe = 0.12 * uStrangeness;
    worldPos.xyz += breathe * vec3(
        sin(uTime * 1.1 + worldPos.z * 0.45 + worldPos.y * 0.3),
        sin(uTime * 0.8 + worldPos.x * 0.35) * 0.6,
        sin(uTime * 1.3 + worldPos.x * 0.4 + worldPos.y * 0.25));

    // The dream melts: everything slumps, drips and sinks into the floor,
    // then (uMelt running back to 0) rises out of it as the next dream.
    if (uMelt > 0.0) {
        // Each ~1.5-unit column drips at its own speed. Hash the un-breathed
        // position so columns don't flicker as the world breathes.
        float h = fract(sin(dot(floor(vWorld.xz * 0.7), vec2(12.9898, 78.233))) * 43758.5453);
        float sag = uMelt * uMelt * (3.0 + 7.0 * h);
        // Tops fall further than bottoms: things slump before they sink.
        worldPos.y -= sag * (0.35 + 0.3 * max(worldPos.y + 1.0, 0.0));
        // The puddle spreads and ripples.
        worldPos.xz += uMelt * 0.8 * vec2(
            sin(worldPos.z * 0.9 + uTime * 2.3),
            cos(worldPos.x * 0.9 + uTime * 1.9));
    }

    vec4 viewPos = uView * worldPos;
    vec4 clipPos = uProj * viewPos;

    if (uVertexSnapAmount > 0.0) {
        float w = clipPos.w;
        vec2 ndc = clipPos.xy / w;
        ndc = floor(ndc / uVertexSnapAmount + 0.5) * uVertexSnapAmount;
        clipPos.xy = ndc * w;
    }

    gl_Position = clipPos;

    if (uLightingMode == 1) {
        float ndotl = max(dot(worldNormal, normalize(uLightDir)), 0.0);
        vLight = uAmbientColor + vec3(ndotl);

        for (int i = 0; i < uPointLightCount; i++) {
            vec3 toLight = uPointLightPos[i] - worldPos.xyz;
            float dist = length(toLight);
            float atten = clamp(1.0 - dist / max(uPointLightRange[i], 0.001), 0.0, 1.0);
            float pointNdotl = max(dot(worldNormal, normalize(toLight)), 0.0);
            vLight += uPointLightColor[i] * uPointLightIntensity[i] * pointNdotl * atten;
        }

        vLight += aColor;
    } else {
        vLight = vec3(1.0);
    }

    float viewDist = length(viewPos.xyz);
    vFogFactor = clamp((viewDist - uFogStart) / max(uFogEnd - uFogStart, 0.001), 0.0, 1.0);
}
