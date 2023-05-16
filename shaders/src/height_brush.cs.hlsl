[[vk::binding(0, 0), vk::image_format("r16f")]]
RWTexture2D<float> heights;

[[vk::push_constant]] struct PC {
    float2 uv;
} pc;

[numthreads(1, 1, 1)]
void main(uint3 GlobalInvocationID : SV_DispatchThreadID) {
    uint w, h;
    heights.GetDimensions(w, h);
    int2 texels = int2(float2(w, h) * pc.uv);
    float height = heights.Load(int3(texels, 0)) + 0.2;
    heights[texels] = height;
}