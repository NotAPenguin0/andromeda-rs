struct VSOutput {
    float4 Position : SV_POSITION;
    float2 UV : UV0;
};

struct HSOutput {
    float4 Position : SV_POSITION;
    float2 UV : UV0;
};

struct ConstantsHSOutput {
    float TessLevelOuter[4] : SV_TessFactor;
    float TessLevelInner[2] : SV_InsideTessFactor;
};

[[vk::push_constant]]
struct PC {
    uint tessellation_factor;
} pc;

// TODO: Tessellation factor based on screen size? 
// (See https://github.com/SaschaWillems/Vulkan/blob/fa0e3485d00c73b0637b914e6510d142ee2e5fdf/data/shaders/hlsl/terraintessellation/terrain.tesc#L41)

ConstantsHSOutput HSConstants(InputPatch<VSOutput, 4> patch, uint InvocationID : SV_PrimitiveID) {
    ConstantsHSOutput output = (ConstantsHSOutput)0;
    output.TessLevelOuter[0] = pc.tessellation_factor;
    output.TessLevelOuter[1] = pc.tessellation_factor;
    output.TessLevelOuter[2] = pc.tessellation_factor;
    output.TessLevelOuter[3] = pc.tessellation_factor;
    output.TessLevelInner[0] = lerp(output.TessLevelOuter[0], output.TessLevelOuter[3], 0.5);
    output.TessLevelInner[1] = lerp(output.TessLevelOuter[2], output.TessLevelOuter[1], 0.5);
    return output;
}

[domain("quad")]
[partitioning("integer")]
[outputtopology("triangle_ccw")]
[outputcontrolpoints(4)]
[patchconstantfunc("HSConstants")]
[maxtessfactor(32.0f)]
HSOutput main(InputPatch<VSOutput, 4> patch, uint InvocationID : SV_OutputControlPointID) {
    HSOutput output = (HSOutput) 0;
    output.Position = patch[InvocationID].Position;
    output.UV = patch[InvocationID].UV;
    return output;
}