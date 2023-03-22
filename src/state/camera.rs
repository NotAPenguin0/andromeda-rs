use glam::{Mat4, Vec3};
use tiny_tokio_actor::{Actor, ActorContext, async_trait, Handler, Message, SystemEvent};

use crate::math;

#[derive(Debug, Actor)]
pub struct Camera {
    position: math::Position,
    rotation: math::Rotation,
    fov: f32,
}

#[derive(Debug, Clone, Copy)]
pub struct CameraMatrix(pub Mat4);

/// Base vectors for the camera's coordinate space.
#[derive(Debug, Clone, Copy)]
pub struct CameraVectors {
    pub front: Vec3,
    pub right: Vec3,
    pub up: Vec3,
}

/// Query camera matrix
#[derive(Debug, Default, Clone, Message)]
#[response(CameraMatrix)]
pub struct QueryCameraMatrix;

#[derive(Debug, Default, Clone, Message)]
#[response(CameraVectors)]
pub struct QueryCameraVectors;

#[derive(Debug, Default, Clone, Message)]
#[response(math::Position)]
pub struct QueryCameraPosition;

#[derive(Debug, Default, Clone, Message)]
#[response(math::Rotation)]
pub struct QueryCameraRotation;

#[derive(Debug, Default, Clone, Message)]
#[response(f32)]
pub struct QueryCameraFOV;

/// Reset camera position to new value
#[derive(Debug, Clone, Message)]
pub struct SetCameraPosition(pub math::Position);
/// Reset camera rotation to new value
#[derive(Debug, Clone, Message)]
pub struct SetCameraRotation(pub math::Rotation);
/// Add value to camera position
#[derive(Debug, Clone, Message)]
pub struct UpdateCameraPosition(pub math::Position);
/// Add value to camera rotation
#[derive(Debug, Clone, Message)]
pub struct UpdateCameraRotation(pub math::Rotation);
/// Add value to camera FOV.
#[derive(Debug, Clone, Message)]
pub struct UpdateCameraFOV(pub f32);

impl From<math::Position> for SetCameraPosition {
    fn from(value: math::Position) -> Self {
        Self(value)
    }
}

impl From<math::Rotation> for SetCameraRotation {
    fn from(value: math::Rotation) -> Self {
        Self(value)
    }
}

impl Default for Camera {
    fn default() -> Self {
        Camera {
            position: Default::default(),
            rotation: Default::default(),
            fov: 90.0,
        }
    }
}

impl Camera {
    fn front(&self) -> Vec3 {
        let cos_pitch = self.rotation.0.x.cos();
        let cos_yaw = self.rotation.0.y.cos();
        let sin_pitch = self.rotation.0.x.sin();
        let sin_yaw = self.rotation.0.y.sin();

        Vec3::new(cos_pitch * cos_yaw, sin_pitch, cos_pitch * sin_yaw).normalize()
    }

    fn right(&self) -> Vec3 {
        self.front().cross(Vec3::new(0.0, 1.0, 0.0)).normalize()
    }

    fn up(&self) -> Vec3 {
        self.right().cross(self.front()).normalize()
    }

    fn clamp_rotation(rot: math::Rotation) -> math::Rotation {
        const MAX_ANGLE: f32 = std::f32::consts::PI / 2.0 - 0.0001;
        const UNBOUNDED: f32 = f32::MAX;
        math::Rotation(rot.0.clamp(
            Vec3::new(-MAX_ANGLE, -UNBOUNDED, 0.0),
            Vec3::new(MAX_ANGLE, UNBOUNDED, 0.0),
        ))
    }
}

#[async_trait]
impl<E> Handler<E, QueryCameraMatrix> for Camera
    where
        E: SystemEvent,
{
    async fn handle(
        &mut self,
        _msg: QueryCameraMatrix,
        _ctx: &mut ActorContext<E>,
    ) -> CameraMatrix {
        let front = self.front();
        let up = self.up();

        CameraMatrix {
            0: Mat4::look_at_rh(self.position.0, self.position.0 + front, up),
        }
    }
}

#[async_trait]
impl<E> Handler<E, QueryCameraVectors> for Camera
    where
        E: SystemEvent,
{
    async fn handle(
        &mut self,
        _msg: QueryCameraVectors,
        _ctx: &mut ActorContext<E>,
    ) -> CameraVectors {
        CameraVectors {
            front: self.front(),
            right: self.right(),
            up: self.up(),
        }
    }
}

#[async_trait]
impl<E> Handler<E, QueryCameraPosition> for Camera
    where
        E: SystemEvent,
{
    async fn handle(
        &mut self,
        _msg: QueryCameraPosition,
        _ctx: &mut ActorContext<E>,
    ) -> math::Position {
        self.position
    }
}

#[async_trait]
impl<E> Handler<E, QueryCameraRotation> for Camera
    where
        E: SystemEvent,
{
    async fn handle(
        &mut self,
        _msg: QueryCameraRotation,
        _ctx: &mut ActorContext<E>,
    ) -> math::Rotation {
        self.rotation
    }
}

#[async_trait]
impl<E> Handler<E, QueryCameraFOV> for Camera
    where
        E: SystemEvent,
{
    async fn handle(&mut self, _msg: QueryCameraFOV, _ctx: &mut ActorContext<E>) -> f32 {
        self.fov
    }
}

#[async_trait]
impl<E> Handler<E, SetCameraPosition> for Camera
    where
        E: SystemEvent,
{
    async fn handle(&mut self, msg: SetCameraPosition, _ctx: &mut ActorContext<E>) -> () {
        self.position = msg.0;
    }
}

#[async_trait]
impl<E> Handler<E, SetCameraRotation> for Camera
    where
        E: SystemEvent,
{
    async fn handle(&mut self, msg: SetCameraRotation, _ctx: &mut ActorContext<E>) -> () {
        self.rotation = Self::clamp_rotation(msg.0);
    }
}

#[async_trait]
impl<E> Handler<E, UpdateCameraPosition> for Camera
    where
        E: SystemEvent,
{
    async fn handle(&mut self, msg: UpdateCameraPosition, _ctx: &mut ActorContext<E>) -> () {
        self.position.0 += msg.0.0;
    }
}

#[async_trait]
impl<E> Handler<E, UpdateCameraRotation> for Camera
    where
        E: SystemEvent,
{
    async fn handle(&mut self, msg: UpdateCameraRotation, _ctx: &mut ActorContext<E>) -> () {
        self.rotation.0 += msg.0.0;
        self.rotation = Self::clamp_rotation(self.rotation);
    }
}

#[async_trait]
impl<E> Handler<E, UpdateCameraFOV> for Camera
    where
        E: SystemEvent,
{
    async fn handle(&mut self, msg: UpdateCameraFOV, _ctx: &mut ActorContext<E>) -> () {
        self.fov += msg.0;
        self.fov = self.fov.clamp(0.001, 180.0);
    }
}
