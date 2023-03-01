struct VSInput {
	[[vk::location(0)]] float3 Position : POSITION0;
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
    output.UV = float2(0.0, 0.0);
    output.Position = mul(projection_view, float4(input.Position, 1.0));
    // Invert y because Vulkan.
    output.Position.y = 1.0 - output.Position.y;
    return output;
}