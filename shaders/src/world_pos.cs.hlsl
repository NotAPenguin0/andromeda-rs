[[vk::combinedImageSampler, vk::binding(0, 0)]]
Texture2D<float> depth;

[[vk::combinedImageSampler, vk::binding(0, 0)]]
SamplerState smp;

[[vk::binding(1, 0)]]
RWStructuredBuffer<float4> out_data;

[[vk::binding(2, 0)]]
cbuffer Camera
{
    float4x4 inv_projection;
    float4x4 inv_view;
};

[[vk::push_constant]]
struct PC {
    float2 screen_pos;
    uint idx;
} pc;

float sample_depth(float2 uv) {
    return depth.SampleLevel(smp, uv, 0);
}

[numthreads(1, 1, 1)]
void main(uint3 GlobalInvocationID : SV_DispatchThreadID) {
    uint width, height;
    depth.GetDimensions(width, height);
    float2 uv = pc.screen_pos / float2(width, height);
    float x = uv.x * 2 - 1;
    float y = uv.y * 2 - 1;
    float z = sample_depth(uv);
    float4 projected_pos = float4(x, y, z, 1.0f);
    float4 unprojected = mul(inv_projection, projected_pos);
    float4 viewspace_pos = float4(unprojected.xyz / unprojected.w, 1.0);
    float4 worldspace_pos = mul(inv_view, viewspace_pos);
    out_data[pc.idx] = worldspace_pos;
}