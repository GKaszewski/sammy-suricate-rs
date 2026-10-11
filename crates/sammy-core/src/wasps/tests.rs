use super::*;

#[test]
fn weak_door_stages() {
    let mut d = WeakDoor { hp: 4.0 };
    assert_eq!(d.damage(), DoorDamage::Untouched);
    d.hit(1.0);
    assert_eq!(d.damage(), DoorDamage::Morph("doorwk2.wmb"));
    d.hit(3.0);
    assert_eq!(d.damage(), DoorDamage::Destroyed);
}

#[test]
fn ammo_type_change_drops_old_ammo() {
    let mut game = GameState { ammo: 5, ammo_type: 2, ..Default::default() };
    AmmoPickup { kind: 3, amount: 2, poison: 1.0 }.take(&mut game);
    assert_eq!((game.ammo, game.ammo_type), (2, 3));
}
