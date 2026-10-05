// Vertex stage for `MaterialVariant::LitNormals`.
//
// Same vertex format, vertex snapping and fog as `mesh.vert`, but lighting
// moves to the fragment stage: a normal map perturbs the normal per pixel,
// so per-vertex lighting would throw that detail away.
//
// There is deliberately no tangent attribute. `engine::mesh::Vertex` is
// position/normal/uv only, so the fragment stage derives the tangent frame
// from screen-space derivatives instead (see `cotangentFrame` there).

layout (location = 0) in vec3 aPos;
layout (location = 1) in vec3 aNormal;
layout (location = 2) in vec2 aUV;

uniform mat4 uModel;
uniform mat4 uView;
uniform mat4 uProj;
uniform float uVertexSnapAmount;
uniform float uFogStart;
uniform float uFogEnd;

#ifdef AFFINE_UV
noperspective out vec2 vUV;
#else
out vec2 vUV;
#endif
out vec3 vWorldPos;
out vec3 vWorldNormal;
out float vFogFactor;

void main() {
    vec4 worldPos = uModel * vec4(aPos, 1.0);
    vec4 viewPos = uView * worldPos;
    vec4 clipPos = uProj * viewPos;

    if (uVertexSnapAmount > 0.0) {
        float w = clipPos.w;
        vec2 ndc = clipPos.xy / w;
        ndc = floor(ndc / uVertexSnapAmount + 0.5) * uVertexSnapAmount;
        clipPos.xy = ndc * w;
    }

    gl_Position = clipPos;
    vUV = aUV;
    vWorldPos = worldPos.xyz;
    // Inverse-transpose keeps normals perpendicular under non-uniform scale.
    vWorldNormal = mat3(transpose(inverse(uModel))) * aNormal;

    float viewDist = length(viewPos.xyz);
    vFogFactor = clamp((viewDist - uFogStart) / max(uFogEnd - uFogStart, 0.001), 0.0, 1.0);
}
