struct PS_INPUT {
    [[vk::location(0)]] float2 UV : UV0;
    [[vk::location(1)]] float3 Normal : NORMAL0;
};

[[vk::binding(2, 0)]]
cbuffer Lighting {
    float3 sun_dir;
};

[[vk::combinedImageSampler, vk::binding(3, 0)]]
Texture2D<float4> normal_map;

[[vk::combinedImageSampler, vk::binding(3, 0)]]
SamplerState smp;

[[vk::combinedImageSampler, vk::binding(4, 0)]]
Texture2D<float4> diffuse_map;

[[vk::combinedImageSampler, vk::binding(4, 0)]]
SamplerState color_smp;

float4 main(PS_INPUT input) : SV_TARGET {
    float3 normal = normal_map.SampleLevel(smp, input.UV, 0.0).rgb;
    // remap back to [-1, 1]
    normal = normal * 2.0 - float3(1.0, 1.0, 1.0);
    float diff = max(dot(normal, -sun_dir), 0.0);
    // TODO: this pow is not correct, just for this one texture
    float4 color = pow(diffuse_map.Sample(color_smp, input.UV).rgba, 2.2);
    return float4(color.rgb * diff, 1.0);
}