#include "decal.hlsl"

static const float PI = 3.1415926535;

// returns the weight for the brush in function of x in [0..1]
float weight_function(float x) {
    // Gaussian
    float sigma = pc.data[0];
    static const float SQRT2PI = 2.50662827463;
    float w = 1.0 / (sigma * SQRT2PI);
    float p = (x / sigma) * (x / sigma);
    return w * exp(-0.5 * p);
}

float4 main(PS_INPUT input, float4 frag_pos

: SV_Position) : SV_TARGET {
float2 uv = decal_uv(frag_pos);
float2 centered_uv = uv * 2.0 - 1.0;
// Discard everything outside the brush area
float distance = length(centered_uv);
if (distance >= 1.0) {
return float4(0.0, 0.0, 0.0, 0.0);
}

// We will use our weight function to color the decal
float weight = weight_function(distance);
return float4(1.0, 0.0, 0.0, 1.0) *
weight;
}