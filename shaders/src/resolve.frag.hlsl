struct PS_INPUT {
    [[vk::location(0)]] float2 UV : UV0;
};

float max3(float x, float y, float z) {
    return max(x, max(y, z));
}

float3 tonemap(float3 c) {
    return c * rcp(max3(c.r, c.g, c.b) + 1.0);
}

float3 tonemap_weighted(float3 c, float3 w) {
    return c * (w * rcp(max3(c.r, c.g, c.b) + 1.0));
}

float3 inverse_tonemap(float3 c) {
    return c * rcp(1.0 - max3(c.r, c.g, c.b));
}

[[vk::combinedImageSampler, vk::binding(0, 0)]]
Texture2DMS<float4> hdr_input;

[[vk::combinedImageSampler, vk::binding(0, 0)]]
SamplerState smp;

[[vk::push_constant]]
struct PC {
    uint samples;
} pc;

float4 main(in PS_INPUT input) : SV_TARGET {
    float weight = float3(1.0 / pc.samples, 1.0 / pc.samples, 1.0 / pc.samples);
    float w, h, _samples;
    hdr_input.GetDimensions(w, h, _samples);
    int2 texels = input.UV * int2(w, h);
    float4 acc = (float4) 0;
    for (uint i = 0; i < pc.samples; ++i) {
        float4 color = hdr_input.Load(texels, i);
        acc += float4(tonemap_weighted(color.rgb, weight), color.a);
    }
    return float4(inverse_tonemap(acc.rgb), acc.a);
}