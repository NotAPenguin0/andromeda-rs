[[vk::binding(0, 0), vk::image_format("r16f")]]
RWTexture2D<float> tex;

[[vk::push_constant]] struct PC {
    float2 uv;
    uint size;
} pc;

float sample_tex(int x, int y, uint width, uint height) {
    x = (float) clamp(x, 0, width);
    y = (float) clamp(y, 0, height);
    return tex.Load(int3(x, y, 0));
}

float sample_at_uv(float2 uv, uint width, uint height) {
    int2 texel = int2(float2(width, height) * uv);
    return sample_tex(texel.x, texel.y, width, height);
}

bool inside_patch_rect(int2 center, int2 offset) {
    return abs(offset.x) <= pc.size / 2 && abs(offset.y) <= pc.size / 2;
}

static const float PI = 3.1415926535;
static const int BLUR_SAMPLES = 35;
static const float SIGMA = float(BLUR_SAMPLES) * 0.25;

float gaussian(float2 i) {
    i = i / SIGMA;
    return exp( -.5* dot(i, i) ) / ( 2 * PI * SIGMA * SIGMA );
}

[numthreads(16, 16, 1)]
void main(uint3 GlobalInvocationID : SV_DispatchThreadID) {
    uint width, height;
    tex.GetDimensions(width, height);
    int2 center = int2(float2(width, height) * pc.uv);
    int2 offset = int2(GlobalInvocationID.xy) - int(pc.size / 2);
    int2 texel = center + offset;
    if (texel.x < 0 || texel.y < 0 || texel.x >= width || texel.y >= height) {
        return;
    }

    if (!inside_patch_rect(center, offset)) {
        return;
    }

    float2 scale = 1.0 / float2(pc.size, pc.size);
    // First collect all samples, since we need to properly synchronize reading and writing to the texture
    float samples[BLUR_SAMPLES * BLUR_SAMPLES];
    for (int i = 0; i < BLUR_SAMPLES * BLUR_SAMPLES; ++i) {
        float2 direction = float2(i % BLUR_SAMPLES, i / float(BLUR_SAMPLES)) - float(BLUR_SAMPLES) / 2;
        samples[i] = sample_at_uv(pc.uv + scale * direction, width, height);
    }
    // TODO: Check if this is the best barrier to use here
    AllMemoryBarrier();

    // With all reads completed (guaranteed by previous barrier), we can now
    // write to the texture
    float output = 0.0;
    float accum = 0.0;
    for (int i = 0; i < BLUR_SAMPLES * BLUR_SAMPLES; ++i) {
        float2 direction = float2(i % BLUR_SAMPLES, i / float(BLUR_SAMPLES)) - float(BLUR_SAMPLES) / 2;
        float weight = gaussian(direction);
        output += samples[i] * weight;
        accum += weight;
    }

    output /= accum;
}