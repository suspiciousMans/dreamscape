// Fragment stage for `MaterialVariant::LitNormals`: per-pixel lighting
// with an optional tangent-space normal map and a Blinn-Phong specular
// term driven by the material's roughness/metallic.
//
// The light model deliberately matches `mesh.vert` — white directional
// light + `uAmbientColor` + up to four linear-falloff point lights — so a
// normal-mapped object sits in the same scene as everything else instead
// of looking lit by a different world.

#ifdef AFFINE_UV
noperspective in vec2 vUV;
#else
in vec2 vUV;
#endif
in vec3 vWorldPos;
in vec3 vWorldNormal;
in vec3 vBakedLight;
in float vFogFactor;

out vec4 FragColor;

uniform sampler2D uTex;
uniform sampler2D uNormalMap;
uniform int uHasNormalMap;
uniform float uRoughness;
uniform float uMetallic;

uniform vec3 uCameraPos;
uniform vec3 uLightDir;
uniform vec3 uAmbientColor;
uniform int uLightingMode;
uniform vec3 uFogColor;

uniform vec3 uPointLightPos[4];
uniform vec3 uPointLightColor[4];
uniform float uPointLightIntensity[4];
uniform float uPointLightRange[4];
uniform int uPointLightCount;

// Builds a tangent frame from screen-space derivatives of position and UV
// (Schüler, "Normal Mapping Without Precomputed Tangents"). Handles
// mirrored UVs for free, since the frame is rebuilt per pixel.
mat3 cotangentFrame(vec3 N, vec3 p, vec2 uv) {
    vec3 dp1 = dFdx(p);
    vec3 dp2 = dFdy(p);
    vec2 duv1 = dFdx(uv);
    vec2 duv2 = dFdy(uv);

    vec3 dp2perp = cross(dp2, N);
    vec3 dp1perp = cross(N, dp1);
    vec3 T = dp2perp * duv1.x + dp1perp * duv2.x;
    vec3 B = dp2perp * duv1.y + dp1perp * duv2.y;

    // max(…, tiny) guards against UV-less meshes (all derivatives zero).
    float invmax = inversesqrt(max(max(dot(T, T), dot(B, B)), 1e-12));
    return mat3(T * invmax, B * invmax, N);
}

// Diffuse + specular contribution of one light arriving from direction L.
vec3 shade(vec3 N, vec3 V, vec3 L, vec3 albedo, vec3 specColor, float shininess) {
    float ndotl = max(dot(N, L), 0.0);
    vec3 H = normalize(L + V);
    float spec = pow(max(dot(N, H), 0.0), shininess) * ndotl;
    return albedo * ndotl + specColor * spec;
}

void main() {
    vec4 texColor = texture(uTex, vUV);

    if (uLightingMode != 1) {
        // Unlit profile: mirror mesh.vert's `vLight = vec3(1.0)`.
        FragColor = vec4(mix(texColor.rgb, uFogColor, vFogFactor), texColor.a);
        return;
    }

    vec3 N = normalize(vWorldNormal);
    if (uHasNormalMap == 1) {
        vec3 tangentNormal = texture(uNormalMap, vUV).rgb * 2.0 - 1.0;
        // Maps are expected in the common OpenGL convention (green = up in
        // the image, Blender/Substance default). `GpuTexture` uploads rows
        // without a vertical flip, so image-up runs toward *decreasing* v
        // here, and green must be negated to match. A DirectX-style map
        // (green = down) needs the opposite: drop this line for it.
        tangentNormal.y = -tangentNormal.y;
        N = normalize(cotangentFrame(N, vWorldPos, vUV) * tangentNormal);
    }

    vec3 V = normalize(uCameraPos - vWorldPos);
    float roughness = clamp(uRoughness, 0.0, 1.0);
    float metallic = clamp(uMetallic, 0.0, 1.0);

    // Metals have no diffuse and tint their highlight with the albedo;
    // dielectrics reflect a dim, uncoloured ~4%.
    vec3 albedo = texColor.rgb * (1.0 - metallic);
    vec3 specColor = mix(vec3(0.04), texColor.rgb, metallic);
    // Rough -> broad dull highlight (2), smooth -> tight sharp one (2048).
    float shininess = exp2(10.0 * (1.0 - roughness) + 1.0);

    vec3 color = texColor.rgb * uAmbientColor;
    color += shade(N, V, normalize(uLightDir), albedo, specColor, shininess);
    // Baked static lights (`Sandbox::bake_static_lighting`, same as
    // mesh.vert's `vLight += aColor`). Baked per vertex from the geometric
    // normal, so the normal map can't perturb it; zero if never baked.
    color += albedo * vBakedLight;

    for (int i = 0; i < uPointLightCount; i++) {
        vec3 toLight = uPointLightPos[i] - vWorldPos;
        float dist = length(toLight);
        float atten = clamp(1.0 - dist / max(uPointLightRange[i], 0.001), 0.0, 1.0);
        vec3 radiance = uPointLightColor[i] * uPointLightIntensity[i] * atten;
        color += radiance * shade(N, V, toLight / max(dist, 0.0001), albedo, specColor, shininess);
    }

    FragColor = vec4(mix(color, uFogColor, vFogFactor), texColor.a);
}
