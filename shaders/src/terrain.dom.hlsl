[[vk::binding(0, 0)]]
cbuffer Camera {
    float4x4 projection_view;
};

struct HSOutput {
    float4 Position : SV_POSITION;
};

struct ConstantsHSOutput {
    float TessLevelOuter[4] : SV_TessFactor;
    float TessLevelInner[2] : SV_InsideTessFactor;
};

struct DSOutput {
    float4 Position : SV_POSITION;
};

[domain("quad")]
DSOutput main(ConstantsHSOutput input, float2 TessCoord : SV_DomainLocation, const OutputPatch<HSOutput, 4> patch) {
    DSOutput output = (DSOutput) 0;

    float4 pos1 = lerp(patch[0].Position, patch[1].Position, TessCoord.x);
    float4 pos2 = lerp(patch[3].Position, patch[2].Position, TessCoord.x);
    float4 position = lerp(pos1, pos2, TessCoord.y);
    // TODO: Set height here by sampling our heightmap
    output.Position = mul(projection_view, position);
    return output;
}