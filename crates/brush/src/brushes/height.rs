use anyhow::Result;
use assets::{Heightmap, NormalMap};
use glam::Vec2;
use phobos::domain::All;
use phobos::{
    vk, CommandBuffer, ComputeCmdBuffer, IncompleteCmdBuffer, IncompleteCommandBuffer,
    PipelineStage, Sampler,
};

const SIZE: u32 = 256;

fn record_update_normals<'q>(
    cmd: IncompleteCommandBuffer<'q, All>,
    uv: Vec2,
    sampler: &Sampler,
    heights: &Heightmap,
    normals: &NormalMap,
) -> Result<IncompleteCommandBuffer<'q, All>> {
    // Transition the normal map for writing
    let cmd = cmd.transition_image(
        &normals.image.image.view,
        PipelineStage::TOP_OF_PIPE,
        PipelineStage::COMPUTE_SHADER,
        vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
        vk::ImageLayout::GENERAL,
        vk::AccessFlags2::NONE,
        vk::AccessFlags2::MEMORY_READ | vk::AccessFlags2::MEMORY_WRITE,
    );
    const NORMAL_SIZE: u32 = SIZE + 8;
    let cmd = cmd.bind_compute_pipeline("normal_recompute")?;
    let cmd = cmd
        .bind_storage_image(0, 0, &normals.image.image.view)?
        .bind_sampled_image(0, 1, &heights.image.image.view, sampler)?
        .push_constant(vk::ShaderStageFlags::COMPUTE, 0, &uv)
        .push_constant(vk::ShaderStageFlags::COMPUTE, 8, &NORMAL_SIZE)
        .dispatch(
            (NORMAL_SIZE as f32 / 16.0f32).ceil() as u32,
            (NORMAL_SIZE as f32 / 16.0f32).ceil() as u32,
            1,
        )?;
    // Transition the normal map back to ShaderReadOnlyOptimal for drawing
    let cmd = cmd.transition_image(
        &normals.image.image.view,
        PipelineStage::COMPUTE_SHADER,
        PipelineStage::BOTTOM_OF_PIPE,
        vk::ImageLayout::GENERAL,
        vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
        vk::AccessFlags2::MEMORY_READ | vk::AccessFlags2::MEMORY_WRITE,
        vk::AccessFlags2::NONE,
    );
    Ok(cmd)
}

fn record_height_blur<'q>(
    cmd: IncompleteCommandBuffer<'q, All>,
    uv: Vec2,
    heights: &Heightmap,
) -> Result<IncompleteCommandBuffer<'q, All>> {
    const BLUR_SIZE: u32 = SIZE + 4;
    let cmd = cmd.bind_compute_pipeline("blur_rect")?;
    let cmd = cmd
        .bind_storage_image(0, 0, &heights.image.image.view)?
        .push_constant(vk::ShaderStageFlags::COMPUTE, 0, &uv)
        .push_constant(vk::ShaderStageFlags::COMPUTE, 8, &BLUR_SIZE)
        .dispatch(
            (BLUR_SIZE as f32 / 16.0f32).ceil() as u32,
            (BLUR_SIZE as f32 / 16.0f32).ceil() as u32,
            1,
        )?;
    Ok(cmd)
}

pub fn record_update_commands(
    cmd: IncompleteCommandBuffer<All>,
    uv: Vec2,
    sampler: &Sampler,
    heights: &Heightmap,
    normals: &NormalMap,
) -> Result<CommandBuffer<All>> {
    // We are going to write to this image in a compute shader, so submit a barrier for this first.
    let cmd = cmd.transition_image(
        &heights.image.image.view,
        PipelineStage::TOP_OF_PIPE,
        PipelineStage::COMPUTE_SHADER,
        vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
        vk::ImageLayout::GENERAL,
        vk::AccessFlags2::NONE,
        vk::AccessFlags2::MEMORY_READ | vk::AccessFlags2::MEMORY_WRITE,
    );
    // Bind the pipeline we will use to update the heightmap
    let cmd = cmd.bind_compute_pipeline("height_brush")?;
    // Bind the image to the descriptor, push our uvs to the shader and dispatch our compute shader
    let cmd = cmd
        .bind_storage_image(0, 0, &heights.image.image.view)?
        .push_constant(vk::ShaderStageFlags::COMPUTE, 0, &uv)
        .push_constant(vk::ShaderStageFlags::COMPUTE, 8, &SIZE)
        .dispatch(SIZE / 16, SIZE / 16, 1)?;
    // Add a barrier to synchronize with the blur shader
    let cmd = cmd.memory_barrier(
        PipelineStage::COMPUTE_SHADER,
        vk::AccessFlags2::MEMORY_READ | vk::AccessFlags2::MEMORY_WRITE,
        PipelineStage::COMPUTE_SHADER,
        vk::AccessFlags2::MEMORY_READ | vk::AccessFlags2::MEMORY_WRITE,
    );
    // Transition back to ShaderReadOnlyOptimal for drawing. This also synchronizes access to the heightmap
    // with the normal map calculation shader
    let cmd = cmd.transition_image(
        &heights.image.image.view,
        PipelineStage::COMPUTE_SHADER,
        PipelineStage::COMPUTE_SHADER,
        vk::ImageLayout::GENERAL,
        vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
        vk::AccessFlags2::MEMORY_READ | vk::AccessFlags2::MEMORY_WRITE,
        vk::AccessFlags2::SHADER_SAMPLED_READ,
    );
    let cmd = record_update_normals(cmd, uv, sampler, heights, normals)?;
    cmd.finish()
}
