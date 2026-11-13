/// O que o corpo quer fazer neste tick, já decidido pela aplicação
/// (inclusive se o voo é permitido). Não é um tipo de rede.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct MoveIntent {
    /// -1..1, trás..frente.
    pub forward: f32,
    /// -1..1, esquerda..direita.
    pub right: f32,
    /// Radianos, rotação em torno de Y (direção de "frente").
    pub yaw: f32,
    /// Pular, se estiver no chão (só andando).
    pub jump: bool,
    /// Voo livre: sem gravidade nem colisão.
    pub fly: bool,
    /// -1..1, desce..sobe (só voando).
    pub fly_vertical: f32,
}

impl MoveIntent {
    /// Andar a toda velocidade na direção horizontal `dir` (x, z), pulando
    /// se `jump`. Direção nula fica parado.
    pub fn walk_towards(dir: [f32; 2], jump: bool) -> Self {
        let moving = dir != [0.0, 0.0];
        Self {
            forward: if moving { 1.0 } else { 0.0 },
            // "Frente" com yaw = 0 é -Z; ver `horizontal_dir`.
            yaw: if moving {
                (-dir[0]).atan2(-dir[1])
            } else {
                0.0
            },
            jump,
            ..Default::default()
        }
    }

    /// Direção horizontal (x, z), normalizada se passar de 1.
    pub(crate) fn horizontal_dir(&self) -> [f32; 2] {
        let (sin, cos) = self.yaw.sin_cos();
        let forward = [-sin, -cos];
        let right = [cos, -sin];
        let mut dir = [
            forward[0] * self.forward + right[0] * self.right,
            forward[1] * self.forward + right[1] * self.right,
        ];
        let len = (dir[0] * dir[0] + dir[1] * dir[1]).sqrt();
        if len > 1.0 {
            dir.iter_mut().for_each(|c| *c /= len);
        }
        dir
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn walk_towards_points_where_asked() {
        for dir in [[1.0, 0.0], [0.0, -1.0], [-0.6, 0.8]] {
            let d = MoveIntent::walk_towards(dir, false).horizontal_dir();
            assert!((d[0] - dir[0]).abs() < 1e-5 && (d[1] - dir[1]).abs() < 1e-5);
        }
        assert_eq!(
            MoveIntent::walk_towards([0.0, 0.0], false).horizontal_dir(),
            [0.0, 0.0]
        );
    }
}
