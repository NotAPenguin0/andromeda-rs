struct PS_INPUT {
    [[vk::location(0)]] float2 UV : UV0;
    [[vk::location(1)]] float3 WorldPos : WPOS0;
};

[[vk::binding(2, 0)]]
cbuffer Lighting {
    float4 sun_dir;
};

[[vk::binding(3, 0)]]
cbuffer Util {
    float4 mouse_world_pos;
    float2 mouse_uv_pos;
    uint decal_texel_radius;
};

[[vk::combinedImageSampler, vk::binding(4, 0)]]
Texture2D<float4> normal_map;

[[vk::combinedImageSampler, vk::binding(4, 0)]]
SamplerState smp;

[[vk::combinedImageSampler, vk::binding(5, 0)]]
Texture2D<float4> diffuse_map;

[[vk::combinedImageSampler, vk::binding(5, 0)]]
SamplerState color_smp;

float4 main(PS_INPUT input) : SV_TARGET {
    // Assumption: The normal map has the same resolution as the heightmap
    uint width, height;
    normal_map.GetDimensions(width, height);

    float3 normal = normal_map.SampleLevel(smp, input.UV, 0.0).rgb;
    // remap back to [-1, 1]
    normal = normal * 2.0 - float3(1.0, 1.0, 1.0);
    float diff = max(dot(normal, -sun_dir), 0.0);
    float4 color = diffuse_map.Sample(color_smp, input.UV).rgba;
    float3 pos_plane = float3(mouse_world_pos.x, 0, mouse_world_pos.z);
    float3 world_pos_plane  = float3(input.WorldPos.x, 0, input.WorldPos.z);
    // Only try to draw decal if it is active (radius > 0)
    if (decal_texel_radius > 0) {
        float2 mouse_texel_pos = float2(width, height) * mouse_uv_pos;
        float2 frag_texel_pos = float2(width, height) * input.UV;
        float distance = length(mouse_texel_pos - frag_texel_pos);
        if (distance < decal_texel_radius) {
            return float4(0.4, 0.0, 0.0, 1.0);
        }
    }
    return float4(color.rgb * diff, 1.0);
}