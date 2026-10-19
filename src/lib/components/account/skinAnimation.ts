import type { PlayerObject } from "skinview3d";

const ease = (value: number) => {
  const t = Math.max(0, Math.min(1, value));
  return t * t * (3 - 2 * t);
};

// Soft entrances, a short hold, and soft exits keep gestures from feeling mechanical.
const gesture = (time: number, start: number, hold: number, end: number) =>
  ease((time - start) / (hold - start)) * (1 - ease((time - hold) / (end - hold)));

export function applySkinPose(
  player: PlayerObject,
  time: number,
  launchTime = Infinity,
  reducedMotion = false,
) {
  const t = reducedMotion ? 0 : time;
  const breath = Math.sin(t * 1.65);
  const weight = Math.sin(t * 0.57);
  const cycle = t % 19;
  const glance = gesture(cycle, 2, 3.4, 6.5) - gesture(cycle, 10, 12, 15.5) * 0.8;
  const armRelax = gesture(cycle, 6, 7.5, 10);
  const { head, body, leftArm, rightArm, leftLeg, rightLeg } = player.skin;

  head.rotation.set(
    -0.035 + breath * 0.018 + Math.sin(t * 0.83) * 0.025,
    0.07 + glance * 0.3,
    -0.018 - glance * 0.045 + weight * 0.012,
  );
  body.rotation.set(breath * 0.012, glance * 0.035, -0.012 + weight * 0.022);
  leftArm.rotation.set(
    0.035 + Math.sin(t * 1.2 + 0.4) * 0.035 - armRelax * 0.16,
    armRelax * 0.08,
    0.075 + breath * 0.015 - weight * 0.018 + armRelax * 0.065,
  );
  rightArm.rotation.set(
    -0.11 + Math.sin(t * 1.12 + 2.1) * 0.04,
    -armRelax * 0.035,
    -0.105 - Math.sin(t * 1.65 - 0.35) * 0.018 - weight * 0.018,
  );
  leftLeg.rotation.set(0.025 + weight * 0.015, 0, -0.014);
  rightLeg.rotation.set(-0.025 - weight * 0.015, 0, 0.014);
  player.position.y = breath * 0.035;
  player.rotation.set(0, -0.22 + glance * 0.025, weight * 0.008);
  player.cape.rotation.x = Math.PI * 0.06 + Math.sin(t * 1.65 - 0.6) * 0.014;

  if (reducedMotion || launchTime < 0 || launchTime >= 3.2) return;

  // One greeting per launch: raise the right arm, wave twice, nod, then settle.
  // Blend into the live idle pose so neither transition snaps to a reset pose.
  const blend = ease(launchTime / 0.55) * (1 - ease((launchTime - 2.25) / 0.95));
  const wave = Math.sin(Math.max(0, launchTime - 0.55) * Math.PI * 3);
  const nod = gesture(launchTime, 0.7, 1.1, 1.65);
  rightArm.rotation.x += (-2.55 - rightArm.rotation.x) * blend;
  rightArm.rotation.z += (-0.35 + wave * 0.18 - rightArm.rotation.z) * blend;
  rightArm.rotation.y += (-0.12 - rightArm.rotation.y) * blend;
  head.rotation.x += (0.02 + nod * 0.16 - head.rotation.x) * blend;
  head.rotation.y += (0.02 - head.rotation.y) * blend;
  head.rotation.z *= 1 - blend;
  body.rotation.y += (-0.06 - body.rotation.y) * blend;
  leftArm.rotation.x += (-0.16 - leftArm.rotation.x) * blend;
  player.cape.rotation.x += blend * 0.025;
}

// A relaxed presentation pose for the wardrobe: inspect the outfit, then look up.
export function applyWardrobePose(player: PlayerObject, time: number, reducedMotion = false) {
  const t = reducedMotion ? 0 : time;
  applySkinPose(player, t, Infinity, reducedMotion);
  const inspect = gesture(t % 14, 2, 4, 7);
  const stretch = gesture(t % 14, 8, 9.5, 12.5);
  player.rotation.y = -0.3 + Math.sin(t * 0.4) * 0.12;
  player.skin.head.rotation.x = 0.015 + inspect * 0.18;
  player.skin.head.rotation.y = -0.08 - inspect * 0.23;
  player.skin.leftArm.rotation.x = -0.16 - inspect * 0.36;
  player.skin.leftArm.rotation.z = 0.13 + inspect * 0.18;
  player.skin.rightArm.rotation.x = -0.08 - stretch * 0.25;
  player.skin.rightArm.rotation.z = -0.14 - stretch * 0.16;
  player.skin.leftLeg.rotation.x = -0.08;
  player.skin.rightLeg.rotation.x = 0.065;
}
