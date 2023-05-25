#include "decal.hlsl"

PS_OUTPUT main(PS_INPUT input, float4 frag_pos

: SV_Position) {
PS_OUTPUT output = (PS_OUTPUT) 0;
float2 uv = decal_uv(frag_pos);
float2 centered_uv = uv * 2.0 - 1.0;
// Discard everything outside the brush area
if (
length(centered_uv)
>= 1.0) {
output.
Color = float4(0.0, 0.0, 0.0, 0.0);
} else {
output.
Color = float4(1.0, 0.0, 0.0, 1.0);
}
write_motion_vectors(input, output
);
return
output;
}