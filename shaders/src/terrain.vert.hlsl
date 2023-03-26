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
    // Do not set height yet, since we can do that at a more granular level in the domain shader
    // Maybe its necessary to do this anyway for proper normals and such? No idea yet.
    float4 position = float4(input.Position.x, 0.0, input.Position.y, 1.0);
    output.Position = position;
    return output;
}