#![no_std]

use bytemuck::{Pod, Zeroable};
use core::f32::consts::PI;
use glam::{vec2, vec3, Vec3, Vec4};
#[cfg(target_arch = "spirv")]
use spirv_std::num_traits::Float;
use spirv_std::spirv;

#[derive(Copy, Clone, Pod, Zeroable)]
#[repr(C)]
pub struct ShaderConstants {
    pub width: u32,
    pub height: u32,
    pub time: f32,
}

#[spirv(fragment)]
pub fn main_fs(vtx_color: Vec3, output: &mut Vec4) {
    *output = Vec4::from((vtx_color, 1.));
}

#[spirv(vertex)]
pub fn main_vs(
    #[spirv(vertex_index)] vert_id: i32,
    #[spirv(descriptor_set = 0, binding = 0, storage_buffer)] constants: &ShaderConstants,
    #[spirv(position)] vtx_pos: &mut Vec4,
    vtx_color: &mut Vec3,
) {
    let speed = 0.4;
    let time = constants.time * speed + vert_id as f32 * (2. * PI * 120. / 360.);
    let position = vec2(f32::sin(time), f32::cos(time));
    *vtx_pos = Vec4::from((position, 0.0, 1.0));

    *vtx_color = [vec3(1., 0., 0.), vec3(0., 1., 0.), vec3(0., 0., 1.)][vert_id as usize % 3];
}
