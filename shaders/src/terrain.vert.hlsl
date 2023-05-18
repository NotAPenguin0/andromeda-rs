struct VSInput {
    [[vk::location(0)]] float2 Position : POSITION0;
    [[vk::location(1)]] float2 UV : UV0;
};

struct VSOutput {
	float4 Position : SV_POSITION;
    [[vk::location(0)]] float2 UV : UV0;
};

[[vk::binding(0, 0)]]
cbuffer Camera {
    float4x4 projection_view;
};

VSOutput main(VSInput input, uint VertexIndex : SV_VertexID) {
    VSOutput output = (VSOutput)0;
    // Do not set height yet, since we can do that at a more granular level in the domain shader
    float4 position = float4(input.Position.x, 0.0, input.Position.y, 1.0);
    output.Position = position;
    output.UV = input.UV;
    return output;
}