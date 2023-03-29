struct Vertex {
    float2 Position;
    float2 UV;
};

[[vk::binding(0, 0)]]
StructuredBuffer<Vertex> vertices;

[[vk::binding(1, 0)]]
RWStructuredBuffer<float> normals;

[[vk::combinedImageSampler, vk::binding(2, 0)]]
Texture2D<half> heightmap;

[[vk::combinedImageSampler, vk::binding(2, 0)]]
SamplerState smp;

[[vk::push_constant]]
struct PC {
    // Number of patches in each direction
    uint patch_resolution;
} pc;

uint offset_index(uint idx, int offset) {
    int index = (int) idx + offset;
    index = clamp(index, 0, pc.patch_resolution - 1);
    return (uint) index;
}

float sample_height(uint x, uint y) {
    float2 uv = vertices[x + y * pc.patch_resolution].UV;
    return (float) heightmap.SampleLevel(smp, uv, 0.0);
}

[numthreads(16, 16, 1)]
void main(uint3 GlobalInvocationID : SV_DispatchThreadID) {
    if (GlobalInvocationID.x >= pc.patch_resolution)
        return;
    if (GlobalInvocationID.y >= pc.patch_resolution) 
        return;
    
    // We calculate the normal using a sobel filter
    // For this, we need to consider the vertices in a 3x3 area around each patch vertex.
    float heights[3][3];
    for (int hx = -1; hx <= 1; hx++) {
        for (int hy = -1; hy <= 1; hy++) {
            heights[hx + 1][hy + 1] = sample_height(
                offset_index(GlobalInvocationID.x, hx),
                offset_index(GlobalInvocationID.y, hy)
            );
        }
    }

    // Now that we have our height samples, we can calculate the normal
    uint index = GlobalInvocationID.x + GlobalInvocationID.y * pc.patch_resolution;
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
    
    normal.y = 1.0;
    normal = normalize(normal);

    normals[3 * index] = normal.x;
    normals[3 * index + 1] = normal.y;
    normals[3 * index + 2] = normal.z;
}