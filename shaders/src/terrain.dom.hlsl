[[vk::binding(0, 0)]]
cbuffer Camera {
    float4x4 projection_view;
};

struct HSOutput {
    float4 Position : SV_POSITION;
    float2 UV : UV0;
};

struct ConstantsHSOutput {
    float TessLevelOuter[4] : SV_TessFactor;
    float TessLevelInner[2] : SV_InsideTessFactor;
};

struct DSOutput {
    float4 Position : SV_POSITION;
    float2 UV : UV0;
};

[[vk::push_constant]]
struct PC
{
    uint tessellation_factor;
    float height_scaling;
} pc;


[[vk::combinedImageSampler, vk::binding(1, 0)]]
Texture2D<half> heightmap;

[[vk::combinedImageSampler, vk::binding(1, 0)]]
SamplerState smp;


[domain("quad")]
DSOutput main(ConstantsHSOutput input, float2 TessCoord : SV_DomainLocation, const OutputPatch<HSOutput, 4> patch) {
    DSOutput output = (DSOutput) 0;

    float4 pos1 = lerp(patch[0].Position, patch[1].Position, TessCoord.x);
    float4 pos2 = lerp(patch[3].Position, patch[2].Position, TessCoord.x);
    float4 position = lerp(pos1, pos2, TessCoord.y);
    
    float2 uv0 = lerp(patch[0].UV, patch[1].UV, TessCoord.x);
    float2 uv1 = lerp(patch[3].UV, patch[2].UV, TessCoord.x);
    float2 uv = lerp(uv0, uv1, TessCoord.y);
    
    position.y = heightmap.SampleLevel(smp, uv, 0.0) * pc.height_scaling;
    output.Position = mul(projection_view, position);
    output.UV = uv;
    return output;
}