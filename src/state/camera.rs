use glam::{Mat4, Vec3};
use tiny_tokio_actor::{Actor, ActorContext, async_trait, Handler, Message, SystemEvent};

#[derive(Debug, Default)]
pub struct Camera {
    position: Vec3,
    rotation: Vec3,
}

#[derive(Debug, Clone)]
pub struct CameraMatrix(pub Mat4);

impl<E> Actor<E> for Camera where E: SystemEvent {}

/// Query camera matrix
#[derive(Debug, Clone)]
pub struct QueryCameraMatrix;
/// Reset camera position to new value
#[derive(Debug, Clone)]
pub struct SetCameraPosition(pub Vec3);
/// Reset camera rotation to new value
#[derive(Debug, Clone)]
pub struct SetCameraRotation(pub Vec3);
/// Add value to camera position
#[derive(Debug, Clone)]
pub struct UpdateCameraPosition(pub Vec3);
/// Add value to camera rotation
#[derive(Debug, Clone)]
pub struct UpdateCameraRotation(pub Vec3);

impl Message for QueryCameraMatrix {
    type Response = CameraMatrix;
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
        let cos_pitch = self.rotation.x.to_radians().cos();
        let cos_yaw = self.rotation.y.to_radians().cos();
        let sin_pitch = self.rotation.x.to_radians().sin();
        let sin_yaw = self.rotation.y.to_radians().sin();

        let front = Vec3::new(cos_pitch * cos_yaw, sin_pitch, cos_pitch * sin_yaw).normalize();
        let right = front.cross(Vec3::new(0.0, 1.0, 0.0)).normalize();
        let up = right.cross(front).normalize();

        CameraMatrix {
            0: Mat4::look_at_rh(self.position, self.position + front, up)
        }
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
        self.position += msg.0;
    }
}

#[async_trait]
impl<E> Handler<E, UpdateCameraRotation> for Camera where E: SystemEvent {
    async fn handle(&mut self, msg: UpdateCameraRotation, _ctx: &mut ActorContext<E>) -> () {
        self.rotation += msg.0;
    }
}