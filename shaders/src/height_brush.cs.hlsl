[[vk::binding(0, 0), vk::image_format("r16f")]]
RWTexture2D<float> heights;

[[vk::push_constant]] struct PC {
    float2 uv;
    float weight;
    uint size;
    // If gaussian, this is sigma
    float weight_param1;
} pc;

static const float PI = 3.1415926535;

// returns the weight for the brush in function of x in [0..1]
float weight_function(float x) {
    // Gaussian
    float sigma = pc.weight_param1;
    static const float SQRT2PI = 2.50662827463;
    float w = 1.0 / (sigma * SQRT2PI);
    float p = (x / sigma) * (x / sigma);
    return w * exp(-0.5 * p);
}

float calculate_weight(float distance) {
    float max_distance = pc.size / 2.0;
    float distance_ratio = min(1.0, distance / max_distance);
    return weight_function(distance_ratio);
}

bool inside_patch_rect(int2 center, int2 offset) {
    return abs(offset.x) <= pc.size / 2 && abs(offset.y) <= pc.size / 2;
}

[numthreads(16, 16, 1)]
void main(uint3 GlobalInvocationID : SV_DispatchThreadID) {
    uint w, h;
    heights.GetDimensions(w, h);
    int2 center = int2(float2(w, h) * pc.uv);
    int2 offset = int2(GlobalInvocationID.xy) - int(pc.size / 2);
    int2 texel = center + offset;
    if (texel.x < 0 || texel.y < 0 || texel.x >= w || texel.y >= h) {
        return;
    }

    if (!inside_patch_rect(center, offset)) {
        return;
    }

    float dist = length(float2(offset));
    float weight = calculate_weight(dist);
    float height = heights.Load(int3(texel, 0)) + weight * pc.weight;
    heights[texel] = height;
}