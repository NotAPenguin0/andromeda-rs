struct PS_INPUT {
    [[vk::location(0)]] float2 UV : UV0;
    [[vk::location(1)]] float4 ClipPos : POS0;
    [[vk::location(2)]] float4 PrevClipPos: POS1;
};

struct PS_OUTPUT {
    [[vk::location(0)]] float4 Color : SV_Target0;
    [[vk::location(1)]] float2 Motion : SV_Target1;
};

[[vk::binding(2, 0)]]
cbuffer Lighting {
    float4 sun_dir;
};

[[vk::combinedImageSampler, vk::binding(3, 0)]]
Texture2D<float4> normal_map;

[[vk::combinedImageSampler, vk::binding(3, 0)]]
SamplerState smp;

[[vk::combinedImageSampler, vk::binding(4, 0)]]
Texture2D<float4> diffuse_map;

[[vk::combinedImageSampler, vk::binding(4, 0)]]
SamplerState color_smp;

PS_OUTPUT main(PS_INPUT input) {
    PS_OUTPUT output = (PS_OUTPUT) 0;

    // Assumption: The normal map has the same resolution as the heightmap
    uint width, height;
    normal_map.GetDimensions(width, height);

    float3 normal = normal_map.SampleLevel(smp, input.UV, 0.0).rgb;
    // remap back to [-1, 1]
    normal = normal * 2.0 - float3(1.0, 1.0, 1.0);
    float diff = max(dot(normal, -sun_dir), 0.0);
    float4 color = diffuse_map.Sample(color_smp, input.UV).rgba;
    output.Color = float4(color.rgb * diff, 1.0);
    output.Motion = input.PrevClipPos.xy / input.PrevClipPos.w - input.ClipPos.xy / input.ClipPos.w;
    return output;
}