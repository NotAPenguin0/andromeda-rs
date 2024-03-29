// Include this shader at the start of your custom decal shader

// You may not add additional bindings

struct PS_INPUT {};

[[vk::binding(0, 0)]]
cbuffer Transform {
    float4x4 projection_view;
    float4x4 inv_projection;
    float4x4 inv_view;
    float4x4 transform;
    float4x4 decal_space_transform;
};

[[vk::combinedImageSampler, vk::binding(1, 0)]]
Texture2D<half> depth_rt;

[[vk::combinedImageSampler, vk::binding(1, 0)]]
SamplerState smp;

[[vk::push_constant]]
struct PC {
    uint vp_width;
    uint vp_height;
    // Additional data you may use
    float data[4];
} pc;

float4 screen_to_world(float4 screen_pos) {
    float4 project_space = mul(inv_projection, screen_pos);
    // Apply perspective division
    float4 view_space = float4(project_space.xyz / project_space.w, 1.0);
    float4 world_space = mul(inv_view, view_space);
    return world_space;
}

// Obtain the decal uv coordinates from a screen space fragment position.
// Will discard the current fragment if it is outside of the decal.
float2 decal_uv(float4 frag_pos) {
    float2 frag_uv = frag_pos.xy / float2(pc.vp_width, pc.vp_height);
    float px_depth = depth_rt.SampleLevel(smp, frag_uv, 0).x;
    // Compute worldspace position of fragment
    float clip_x = frag_uv.x * 2 - 1;
    float clip_y = frag_uv.y * 2 - 1;
    float4 screen_pos = float4(clip_x, clip_y, px_depth, 1.0f);
    float4 world_pos = screen_to_world(screen_pos);
    // Transform worldspace position to decal space position
    float3 decal_pos = mul(decal_space_transform, world_pos).xyz;
    clip(0.5 - abs(decal_pos));
    // Compute decal uvs from position
    return decal_pos.xy + 0.5;
}
