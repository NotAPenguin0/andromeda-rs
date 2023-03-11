#include "color_space.hlsl"

struct PS_INPUT {
    [[vk::location(0)]] float2 UV : UV0;
};


[[vk::combinedImageSampler, vk::binding(0, 0)]]
Texture2D<float4> hdr_input;

[[vk::combinedImageSampler, vk::binding(0, 0)]]
SamplerState smp;


// Clamps a value to [0...1]
float saturate(float x) {
    return max(0.0, min(x, 1.0));
}

// Quadratically eases a value between 0 and 1.
// This implements the easing function at https://easings.net/#easeOutQuad
float ease_out_quadratic(float x) {
    return 1.0 - (x - 1.0) * (x - 1.0);
}

float3 uncharted2_tonemap_partial(float3 x) {
    float A = 0.15f;
    float B = 0.50f;
    float C = 0.10f;
    float D = 0.20f;
    float E = 0.02f;
    float F = 0.30f;
    return ((x*(A*x+C*B)+D*E)/(x*(A*x+B)+D*F))-E/F;
}

float3 uncharted2_tonemap_filmic(float3 v) {
    float exposure_bias = 2.0f;
    float3 curr = uncharted2_tonemap_partial(v * exposure_bias);

    float3 W = float3(11.2f, 11.2f, 11.2f);
    float3 white_scale = float3(1.0f, 1.0f, 1.0f) / uncharted2_tonemap_partial(W);
    return curr * white_scale;
}

// Narkowicz 2015, "ACES Filmic Tone Mapping Curve"
float3 aces_tonemap(float3 x) {
    x *= 0.6; //
    const float a = 2.51;
    const float b = 0.03;
    const float c = 2.43;
    const float d = 0.59;
    const float e = 0.14;
    return clamp((x * (a * x + b)) / (x * (c * x + d) + e), 0.0, 1.0);
}

// Version of ACES tonemap that only operates on a luminance value.
float aces_tonemap(float x) {
    const float a = 2.51;
    const float b = 0.03;
    const float c = 2.43;
    const float d = 0.59;
    const float e = 0.14;
    return clamp((x * (a * x + b)) / (x * (c * x + d) + e), 0.0, 1.0);
}

float4 main(in PS_INPUT input) : SV_TARGET {
    float3 color = hdr_input.Sample(smp, input.UV).rgb;
    float3 xyY = srgb2xyY(color);
    float lum = xyY.b;
    lum = aces_tonemap(lum);
    xyY.b = lum;
    return float4(xyY2srgb(xyY), 1.0);
}