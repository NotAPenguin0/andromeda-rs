struct VSInput {
	[[vk::location(0)]] float2 Position : POSITION0;
};

struct VSOutput {
	float4 Position : SV_POSITION;
};

[[vk::binding(0, 0)]]
cbuffer Camera {
    float4x4 projection_view;
};

VSOutput main(VSInput input, uint VertexIndex : SV_VertexID) {
    VSOutput output = (VSOutput)0;
    float4 position = float4(input.Position.x, 0.0, input.Position.y, 1.0);
    output.Position = mul(projection_view, position);
    return output;
}