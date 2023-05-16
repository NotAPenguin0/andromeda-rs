[[vk::binding(0, 0), vk::image_format("r16f")]]
RWTexture2D<float> heights;

[[vk::push_constant]] struct PC {
    float2 uv;
    uint size;
} pc;

static const float PI = 3.1415926535;

// returns the weight for the brush in function of x in [0..1]
float weight_function(float x) {
    // in/out sin easing
    return 1.0 - (-(cos(PI * x) - 1) / 2);
}

float calculate_weight(float distance) {
    return 1.0;
    float max_distance = sqrt(2) * pc.size / 2.0;
    float distance_ratio = distance / max_distance;
    return weight_function(distance_ratio);
}

[numthreads(16, 16, 1)]
void main(uint3 GlobalInvocationID : SV_DispatchThreadID) {
    uint w, h;
    heights.GetDimensions(w, h);
    int2 texel = int2(float2(w, h) * pc.uv);
    int2 offset = int2(GlobalInvocationID.xy) - int(pc.size / 2);
    texel = texel + offset;
    if (texel.x < 0 || texel.y < 0 || texel.x >= w || texel.y >= h)
        return;
    float dist = length(float2(offset));
    float weight = calculate_weight(dist);
    float height = heights.Load(int3(texel, 0)) + 0.01 * weight;
    heights[texel] = height;
}