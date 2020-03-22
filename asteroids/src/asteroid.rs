use rand::Rng;
use rand_pcg::Pcg32;
use std::f64::consts::PI;

use crate::blast::Blast;
use crate::geometry::{Circle, Point, Polygon, Size};
use crate::iter::EdgesCycleIterator;
use crate::motion::{Collide, Movement, Placement};
use crate::particle::{Dispersion, Particle};

pub struct Impact {
    pub fragments: Vec<Asteroid>,
    pub particles: Vec<Particle>,
}

pub struct Asteroid {
    radius: f64,
    placement: Placement,
    movement: Movement,
    polygon: Vec<Point>,
}

impl Asteroid {
    pub fn new(rng: &mut Pcg32) -> Self {
        let radius: f64 = rng.gen_range(25.0, 55.0);
        Asteroid {
            radius,
            placement: Placement {
                position: Point::new(0.0, 0.0),
                rotation: 0.0,
            },
            movement: Movement {
                velocity: Point::from_polar(
                    rng.gen_range(10.0, 80.0),
                    rng.gen_range(0.0, 2.0 * PI),
                ),
                angular_velocity: rng.gen_range(-1.0, 1.0),
            },
            polygon: Asteroid::shape(rng, radius),
        }
    }

    fn shape(rng: &mut Pcg32, radius: f64) -> Vec<Point> {
        let n: u32 = rng.gen_range((radius / 5.0).floor() as u32, (radius / 4.0).ceil() as u32);
        let angle = (2.0 * PI) / (n as f64);
        (0..n)
            .map(|i| {
                Point::from_polar(
                    radius * rng.gen_range(0.6, 1.0),
                    angle * (i as f64) + angle * rng.gen_range(0.1, 1.0),
                )
            })
            .collect()
    }

    pub fn from_polygon(polygon: &Vec<Point>) -> Self {
        let Circle { center, radius } = Circle::enclose(polygon);
        let polygon = polygon.iter().map(|point| point.sub(&center)).collect();
        Asteroid {
            radius,
            placement: Placement {
                position: center,
                rotation: 0.0,
            },
            movement: Movement::zero(),
            polygon,
        }
    }

    pub fn movement(&self) -> &Movement {
        &self.movement
    }

    pub fn grid(rng: &mut Pcg32, cols: u32, rows: u32) -> Vec<Asteroid> {
        let mut list = Vec::with_capacity((cols * rows) as usize);
        for row in 0..rows {
            for col in 0..cols {
                let mut asteroid = Asteroid::new(rng);
                asteroid.placement.position.x = (100 + col * 200) as f64;
                asteroid.placement.position.y = (100 + row * 200) as f64;
                list.push(asteroid);
            }
        }
        list
    }

    pub fn field(rng: &mut Pcg32, bounds: &Size, count: u32, clearing: f64) -> Vec<Asteroid> {
        let center = bounds.center();
        let mut list = Vec::with_capacity(count as usize);
        for _ in 0..count {
            let mut asteroid = Asteroid::new(rng);
            loop {
                asteroid.placement.position.x = rng.gen_range(0.0, bounds.width);
                asteroid.placement.position.y = rng.gen_range(0.0, bounds.height);

                if clearing == 0.0
                    || clearing + asteroid.radius < center.distance(&asteroid.placement.position)
                {
                    break;
                }
            }
            list.push(asteroid);
        }
        list
    }

    pub fn step(&mut self, dt: f64, bounds: &Size) -> () {
        self.placement
            .apply_movement(&self.movement, dt)
            .wrap_position(bounds);
    }

    pub fn to_path(&self) -> Vec<Point> {
        self.placement.transform_points(&self.polygon)
    }

    pub fn interact_blast(&self, rng: &mut Pcg32, blast: &Blast) -> Option<Impact> {
        if let Some(impact) = blast.impact(self) {
            let mut fragments = Vec::new();
            let mut particles = Dispersion::new(
                impact.point.clone(),
                self.movement().velocity.clone(),
                100.0,
                50.0,
            )
            .burst(rng, (self.radius() / 4.0).ceil() as u32);

            let (head, tail) = blast.endpoints();
            for fragment_boundary in Polygon(&self.boundary()).split(&head, &tail).iter() {
                let mut fragment = Asteroid::from_polygon(fragment_boundary);
                fragment.movement = {
                    let impact_velocity = blast.velocity().normalize().scale(impact.speed);
                    let impact_movement =
                        Movement::from_impulse(fragment.center(), &impact.point, &impact_velocity);
                    let outward_movement = Movement {
                        velocity: (self.center().direction_to(&fragment.center()))
                            .scale(impact.speed),
                        angular_velocity: 0.0,
                    };
                    outward_movement
                        .interpolate(self.movement(), fragment.mass() / self.mass())
                        .add(&impact_movement)
                };

                if fragment.radius() < 18.0 {
                    let mut fragment_particles = Dispersion::new(
                        fragment.center().clone(),
                        fragment.movement().velocity.clone(),
                        impact.speed,
                        impact.speed,
                    )
                    .explode(rng, fragment.boundary().iter().edges_cycle());
                    particles.append(&mut fragment_particles);
                } else {
                    fragments.push(fragment);
                }
            }

            Some(Impact {
                fragments,
                particles,
            })
        } else {
            None
        }
    }
}

impl Collide for Asteroid {
    fn center(&self) -> &Point {
        &self.placement.position
    }
    fn radius(&self) -> f64 {
        self.radius
    }
    fn boundary(&self) -> Vec<Point> {
        self.to_path()
    }
    fn movement(&self) -> &Movement {
        &self.movement
    }
    fn mass(&self) -> f64 {
        self.radius.powi(2)
    }
}
