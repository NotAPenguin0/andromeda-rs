struct VSOutput {
	float4 Position : SV_POSITION;
    [[vk::location(0)]]
    float4 ClipPos : POS0;
    [[vk::location(1)]]
    float4 PrevClipPos : POS1;
};

[[vk::binding(0, 0)]]
cbuffer Transform {
    float4x4 projection_view;
    float4x4 inv_projection;
    float4x4 inv_view;
    float4x4 transform;
    float4x4 decal_space_transform;
    float4x4 prev_pv;
};

// Cube vertices
static const float3 vertices[] = {
    float3(-0.5f,-0.5f,-0.5f),
    float3(-0.5f,-0.5f, 0.5f),
    float3(-0.5f, 0.5f, 0.5f),
    float3(0.5f, 0.5f,-0.5f),
    float3(-0.5f,-0.5f,-0.5f),
    float3(-0.5f, 0.5f,-0.5f),
    float3(0.5f,-0.5f, 0.5f),
    float3(-0.5f,-0.5f,-0.5f),
    float3(0.5f,-0.5f,-0.5f),
    float3(0.5f, 0.5f,-0.5f),
    float3(0.5f,-0.5f,-0.5f),
    float3(-0.5f,-0.5f,-0.5f),
    float3(-0.5f,-0.5f,-0.5f),
    float3(-0.5f, 0.5f, 0.5f),
    float3(-0.5f, 0.5f,-0.5f),
    float3(0.5f,-0.5f, 0.5f),
    float3(-0.5f,-0.5f, 0.5f),
    float3(-0.5f,-0.5f,-0.5f),
    float3(-0.5f, 0.5f, 0.5f),
    float3(-0.5f,-0.5f, 0.5f),
    float3(0.5f,-0.5f, 0.5f),
    float3(0.5f, 0.5f, 0.5f),
    float3(0.5f,-0.5f,-0.5f),
    float3(0.5f, 0.5f,-0.5f),
    float3(0.5f,-0.5f,-0.5f),
    float3(0.5f, 0.5f, 0.5f),
    float3(0.5f,-0.5f, 0.5f),
    float3(0.5f, 0.5f, 0.5f),
    float3(0.5f, 0.5f,-0.5f),
    float3(-0.5f, 0.5f,-0.5f),
    float3(0.5f, 0.5f, 0.5f),
    float3(-0.5f, 0.5f,-0.5f),
    float3(-0.5f, 0.5f, 0.5f),
    float3(0.5f, 0.5f, 0.5f),
    float3(-0.5f, 0.5f, 0.5f),
    float3(0.5f,-0.5f, 0.5)
};

VSOutput main(uint VertexIndex : SV_VertexID) {
    float3 inPosition = vertices[VertexIndex];
    VSOutput output = (VSOutput) 0;
    float4 position = mul(transform, float4(inPosition.xyz, 1.0));
    output.Position = mul(projection_view, position);
    output.ClipPos = output.Position;
    output.PrevClipPos = mul(prev_pv, position);
    return output;
}