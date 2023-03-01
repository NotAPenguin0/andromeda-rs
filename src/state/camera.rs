use glam::{Mat4, Vec3};
use tiny_tokio_actor::{Actor, ActorContext, async_trait, Handler, Message, SystemEvent};
use crate::math;

#[derive(Debug, Default)]
pub struct Camera {
    position: math::Position,
    rotation: math::Rotation,
}

#[derive(Debug, Clone)]
pub struct CameraMatrix(pub Mat4);

impl<E> Actor<E> for Camera where E: SystemEvent {}

/// Query camera matrix
#[derive(Debug, Default, Clone)]
pub struct QueryCameraMatrix;
#[derive(Debug, Default, Clone)]
pub struct QueryCameraPosition;
#[derive(Debug, Default, Clone)]
pub struct QueryCameraRotation;
/// Reset camera position to new value
#[derive(Debug, Clone)]
pub struct SetCameraPosition(pub math::Position);
/// Reset camera rotation to new value
#[derive(Debug, Clone)]
pub struct SetCameraRotation(pub math::Rotation);
/// Add value to camera position
#[derive(Debug, Clone)]
pub struct UpdateCameraPosition(pub math::Position);
/// Add value to camera rotation
#[derive(Debug, Clone)]
pub struct UpdateCameraRotation(pub math::Rotation);

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


impl Message for QueryCameraMatrix {
    type Response = CameraMatrix;
}

impl Message for QueryCameraPosition {
    type Response = math::Position;
}

impl Message for QueryCameraRotation {
    type Response = math::Rotation;
}

impl Message for SetCameraPosition {
    type Response = ();
}

impl Message for SetCameraRotation {
    type Response = ();
}

impl Message for UpdateCameraPosition {
    type Response = ();
}

impl Message for UpdateCameraRotation {
    type Response = ();
}

#[async_trait]
impl<E> Handler<E, QueryCameraMatrix> for Camera where E: SystemEvent {
    async fn handle(&mut self, _msg: QueryCameraMatrix, _ctx: &mut ActorContext<E>) -> CameraMatrix {
        let cos_pitch = self.rotation.0.x.to_radians().cos();
        let cos_yaw = self.rotation.0.y.to_radians().cos();
        let sin_pitch = self.rotation.0.x.to_radians().sin();
        let sin_yaw = self.rotation.0.y.to_radians().sin();

        let front = Vec3::new(cos_pitch * cos_yaw, sin_pitch, cos_pitch * sin_yaw).normalize();
        let right = front.cross(Vec3::new(0.0, 1.0, 0.0)).normalize();
        let up = right.cross(front).normalize();

        CameraMatrix {
            0: Mat4::look_at_rh(self.position.0, self.position.0 + front, up)
        }
    }
}

#[async_trait]
impl<E> Handler<E, QueryCameraPosition> for Camera where E: SystemEvent {
    async fn handle(&mut self, _msg: QueryCameraPosition, _ctx: &mut ActorContext<E>) -> math::Position {
        self.position
    }
}

#[async_trait]
impl<E> Handler<E, QueryCameraRotation> for Camera where E: SystemEvent {
    async fn handle(&mut self, _msg: QueryCameraRotation, _ctx: &mut ActorContext<E>) -> math::Rotation {
        self.rotation
    }
}

#[async_trait]
impl<E> Handler<E, SetCameraPosition> for Camera where E: SystemEvent {
    async fn handle(&mut self, msg: SetCameraPosition, _ctx: &mut ActorContext<E>) -> () {
        self.position = msg.0;
    }
}

#[async_trait]
impl<E> Handler<E, SetCameraRotation> for Camera where E: SystemEvent {
    async fn handle(&mut self, msg: SetCameraRotation, _ctx: &mut ActorContext<E>) -> () {
        self.rotation = msg.0;
    }
}

#[async_trait]
impl<E> Handler<E, UpdateCameraPosition> for Camera where E: SystemEvent {
    async fn handle(&mut self, msg: UpdateCameraPosition, _ctx: &mut ActorContext<E>) -> () {
        self.position.0 += msg.0.0;
    }
}

#[async_trait]
impl<E> Handler<E, UpdateCameraRotation> for Camera where E: SystemEvent {
    async fn handle(&mut self, msg: UpdateCameraRotation, _ctx: &mut ActorContext<E>) -> () {
        self.rotation.0 += msg.0.0;
    }
}