[[vk::binding(0, 0)]]
RWTexture2D<float4> normals;

[[vk::combinedImageSampler, vk::binding(1, 0)]]
Texture2D<half> heightmap;

[[vk::combinedImageSampler, vk::binding(1, 0)]]
SamplerState smp;

[[vk::push_constant]] struct PC {
    float2 uv;
    uint size;
} pc;

float sample_height(int x, int y, uint width, uint height) {
    x = (float) clamp(x, 0, width);
    y = (float) clamp(y, 0, height);
    float2 uv = float2(x, y) / float2((float) width, (float) height);
    return heightmap.SampleLevel(smp, uv, 0.0);
}

bool inside_patch_rect(int2 center, int2 offset) {
    return abs(offset.x) <= pc.size / 2 && abs(offset.y) <= pc.size / 2;
}

[numthreads(16, 16, 1)]
void main(uint3 GlobalInvocationID : SV_DispatchThreadID) {
    uint width, height;
    normals.GetDimensions(width, height);
    int2 center = int2(float2(width, height) * pc.uv);
    int2 offset = int2(GlobalInvocationID.xy) - int(pc.size / 2);
    int2 texel = center + offset;
    if (texel.x < 0 || texel.y < 0 || texel.x >= width || texel.y >= height) {
        return;
    }

    if (!inside_patch_rect(center, offset)) {
        return;
    }

    // We calculate the normal using a sobel filter
    // For this, we need to consider the vertices in a 3x3 area around each point.
    float heights[3][3];
    for (int hx = -1; hx <= 1; hx++) {
        for (int hy = -1; hy <= 1; hy++) {
            heights[hx + 1][hy + 1] = sample_height(texel.x + hx, texel.y + hy, width, height);
        }
    }

    // Now that we have our height samples, we can calculate the normal
    float3 normal = (float3) 0;
    normal.x =
        heights[0][0]
        - heights[2][0]
        + 2.0f * heights[0][1]
        - 2.0f * heights[2][1]
        + heights[0][2]
        - heights[2][2];
    normal.z =
        heights[0][0]
        + 2.0f * heights[1][0]
        + heights[2][0]
        - heights[0][2]
        - 2.0f * heights[1][2]
        - heights[2][2];

    // original code has 0.25 sqrt(1 - x * x - z * z) here, which leads to NaNs
    normal.y = 0.25;
    normal = normalize(normal * float3(2.0, 1.0, 2.0));
    // Remap normal from -1, 1 to [0, 1] before storing
    normal = (normal + float3(1.0, 1.0, 1.0)) / 2.0;
    normals[texel] = float4(normal, 0.0);
}