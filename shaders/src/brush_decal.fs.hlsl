#include "decal.hlsl"

float4 main(PS_INPUT input, float4 frag_pos

: SV_Position) : SV_TARGET {
float2 uv = decal_uv(frag_pos);
float2 centered_uv = uv * 2.0 - 1.0;
// Discard everything outside the brush area
if (
length(centered_uv)
>= 1.0) {
return float4(0.0, 0.0, 0.0, 0.0);
} else {
return float4(1.0, 0.0, 0.0, 1.0);
}
}