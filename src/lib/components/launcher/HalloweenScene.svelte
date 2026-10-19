<script lang="ts">
  import { onMount } from "svelte";
  import type { Group, Mesh, Sprite, Vector3 } from "three";

  let { animated = true }: { animated?: boolean } = $props();
  let host: HTMLDivElement;
  let canvas: HTMLCanvasElement;

  onMount(() => {
    let disposed = false;
    let cleanup = () => {};

    void import("three").then((THREE) => {
      if (disposed) return;

      const renderer = new THREE.WebGLRenderer({ canvas, alpha: true, antialias: true });
      renderer.setPixelRatio(Math.min(window.devicePixelRatio, 2));
      renderer.outputColorSpace = THREE.SRGBColorSpace;
      renderer.toneMapping = THREE.ACESFilmicToneMapping;
      renderer.toneMappingExposure = 1.02;
      renderer.shadowMap.enabled = true;
      renderer.shadowMap.type = THREE.PCFSoftShadowMap;

      const scene = new THREE.Scene();
      scene.fog = new THREE.FogExp2(0x080b16, 0.075);
      const camera = new THREE.PerspectiveCamera(40, 1, 0.1, 40);
      camera.position.set(0, 2.55, 10.8);
      camera.lookAt(0, 0.18, -0.35);

      const ambient = new THREE.HemisphereLight(0x779ddb, 0x09040d, 1.18);
      scene.add(ambient);
      const moonLight = new THREE.DirectionalLight(0xc8ddff, 2.25);
      moonLight.position.set(1.5, 7, 4);
      moonLight.castShadow = true;
      scene.add(moonLight);
      const purpleLight = new THREE.PointLight(0x7e42bd, 10, 12, 2);
      purpleLight.position.set(-2.8, 1.1, 2.5);
      scene.add(purpleLight);

      const groundMaterial = new THREE.MeshStandardMaterial({
        color: 0x15121b,
        roughness: 0.92,
        metalness: 0.05,
      });
      const ground = new THREE.Mesh(new THREE.CylinderGeometry(6.8, 7.1, 0.52, 64), groundMaterial);
      ground.position.y = -1.42;
      ground.scale.z = 0.58;
      ground.receiveShadow = true;
      scene.add(ground);

      const hillMaterial = new THREE.MeshStandardMaterial({
        color: 0x10101a,
        roughness: 1,
      });
      for (const [x, y, z, scaleX, scaleY] of [
        [-4.8, -1.35, -3.8, 4.2, 1.4],
        [0.1, -1.72, -4.35, 5.3, 1.18],
        [4.9, -1.42, -3.55, 4.1, 1.48],
      ] as const) {
        const hill = new THREE.Mesh(new THREE.SphereGeometry(1, 28, 15), hillMaterial);
        hill.position.set(x, y, z);
        hill.scale.set(scaleX, scaleY, 1.2);
        hill.receiveShadow = true;
        scene.add(hill);
      }

      const path = new THREE.Mesh(
        new THREE.PlaneGeometry(2.5, 8.5),
        new THREE.MeshStandardMaterial({ color: 0x30262e, roughness: 1 }),
      );
      path.rotation.x = -Math.PI / 2;
      path.rotation.z = -0.07;
      path.position.set(0.25, -1.145, 0.45);
      path.receiveShadow = true;
      scene.add(path);

      const moon = new THREE.Group();
      const moonBody = new THREE.Mesh(
        new THREE.SphereGeometry(1.18, 38, 24),
        new THREE.MeshBasicMaterial({ color: 0xdbe8f5 }),
      );
      moon.add(moonBody);
      const craterMaterial = new THREE.MeshBasicMaterial({
        color: 0xaebdce,
        transparent: true,
        opacity: 0.36,
        depthWrite: false,
      });
      for (const [x, y, scale] of [
        [-0.36, 0.33, 0.22],
        [0.37, 0.08, 0.16],
        [-0.05, -0.4, 0.13],
        [0.18, 0.52, 0.09],
      ] as const) {
        const crater = new THREE.Mesh(new THREE.CircleGeometry(scale, 20), craterMaterial);
        crater.position.set(x, y, 1.145);
        moon.add(crater);
      }
      moon.position.set(0.72, 3.35, -3.8);
      scene.add(moon);
      const moonHalo = new THREE.PointLight(0xaecfff, 6, 10, 2);
      moonHalo.position.copy(moon.position);
      scene.add(moonHalo);

      const house = new THREE.Group();
      const houseMaterial = new THREE.MeshStandardMaterial({
        color: 0x21112b,
        roughness: 0.94,
      });
      const houseFront = new THREE.Mesh(new THREE.BoxGeometry(1.55, 1.65, 0.72), houseMaterial);
      houseFront.castShadow = true;
      houseFront.receiveShadow = true;
      house.add(houseFront);
      const tower = new THREE.Mesh(new THREE.BoxGeometry(0.67, 2.2, 0.62), houseMaterial);
      tower.position.set(-0.53, 0.45, -0.03);
      tower.castShadow = true;
      house.add(tower);
      const sideWing = new THREE.Mesh(new THREE.BoxGeometry(0.95, 1.22, 0.68), houseMaterial);
      sideWing.position.set(1.03, -0.2, 0.01);
      sideWing.castShadow = true;
      house.add(sideWing);
      const roofMaterial = new THREE.MeshStandardMaterial({
        color: 0x0c0711,
        roughness: 0.88,
      });
      const roof = new THREE.Mesh(new THREE.ConeGeometry(1.2, 0.72, 4), roofMaterial);
      roof.position.set(0.03, 1.05, 0);
      roof.rotation.y = Math.PI / 4;
      roof.castShadow = true;
      house.add(roof);
      const towerRoof = new THREE.Mesh(new THREE.ConeGeometry(0.62, 0.82, 4), roofMaterial);
      towerRoof.position.set(-0.53, 1.82, -0.03);
      towerRoof.rotation.y = Math.PI / 4;
      towerRoof.castShadow = true;
      house.add(towerRoof);
      const wingRoof = new THREE.Mesh(new THREE.ConeGeometry(0.76, 0.56, 4), roofMaterial);
      wingRoof.position.set(1.03, 0.64, 0.01);
      wingRoof.rotation.y = Math.PI / 4;
      wingRoof.castShadow = true;
      house.add(wingRoof);
      const chimney = new THREE.Mesh(new THREE.BoxGeometry(0.22, 0.78, 0.24), roofMaterial);
      chimney.position.set(0.45, 1.18, -0.08);
      chimney.rotation.z = -0.045;
      chimney.castShadow = true;
      house.add(chimney);
      const windowMaterial = new THREE.MeshBasicMaterial({
        color: 0xff812b,
        transparent: true,
        opacity: 0.82,
      });
      for (const [x, y, scale] of [
        [0.33, 0.32, 1],
        [-0.53, 0.83, 0.74],
        [1.03, -0.08, 0.76],
      ] as const) {
        const litWindow = new THREE.Mesh(
          new THREE.PlaneGeometry(0.3 * scale, 0.42 * scale),
          windowMaterial,
        );
        litWindow.position.set(x, y, 0.365);
        house.add(litWindow);
        const cross = new THREE.Mesh(
          new THREE.BoxGeometry(0.035, 0.43 * scale, 0.02),
          roofMaterial,
        );
        cross.position.set(x, y, 0.378);
        house.add(cross);
      }
      const door = new THREE.Mesh(new THREE.PlaneGeometry(0.38, 0.68), roofMaterial);
      door.position.set(-0.08, -0.48, 0.37);
      house.add(door);
      const houseGlow = new THREE.PointLight(0xff7a1f, 5.5, 5, 2);
      houseGlow.position.set(-0.05, 0.05, 0.8);
      house.add(houseGlow);
      house.position.set(-3.25, -0.3, -2.8);
      house.rotation.y = 0.08;
      house.scale.setScalar(1.02);
      scene.add(house);

      function createFence(x: number, z: number, rotation: number, count: number) {
        const fence = new THREE.Group();
        const material = new THREE.MeshStandardMaterial({ color: 0x211322, roughness: 1 });
        for (let index = 0; index < count; index += 1) {
          const post = new THREE.Mesh(new THREE.BoxGeometry(0.1, 0.92, 0.1), material);
          post.position.set(index * 0.56, 0, 0);
          post.rotation.z = (index % 2 ? -1 : 1) * 0.045;
          post.castShadow = true;
          fence.add(post);
        }
        for (const railY of [-0.18, 0.18]) {
          const rail = new THREE.Mesh(
            new THREE.BoxGeometry((count - 1) * 0.56 + 0.08, 0.09, 0.08),
            material,
          );
          rail.position.set(((count - 1) * 0.56) / 2, railY, 0);
          rail.rotation.z = railY > 0 ? 0.035 : -0.025;
          rail.castShadow = true;
          fence.add(rail);
        }
        fence.position.set(x, -0.72, z);
        fence.rotation.y = rotation;
        scene.add(fence);
      }

      createFence(-4.05, 0.15, -0.12, 4);
      createFence(2.32, -0.42, 0.11, 4);

      const batShape = new THREE.Shape();
      batShape.moveTo(-0.5, 0);
      batShape.quadraticCurveTo(-0.3, 0.28, -0.08, 0.04);
      batShape.quadraticCurveTo(0, -0.04, 0.08, 0.04);
      batShape.quadraticCurveTo(0.3, 0.28, 0.5, 0);
      batShape.quadraticCurveTo(0.28, 0.08, 0.12, -0.13);
      batShape.lineTo(0, -0.03);
      batShape.lineTo(-0.12, -0.13);
      batShape.quadraticCurveTo(-0.28, 0.08, -0.5, 0);
      const batGeometry = new THREE.ShapeGeometry(batShape);
      const batMaterial = new THREE.MeshBasicMaterial({
        color: 0x120916,
        side: THREE.DoubleSide,
      });
      const bats: { mesh: Mesh; baseX: number; baseY: number; speed: number }[] = [];
      for (const [baseX, baseY, scale, speed] of [
        [2.05, 3.25, 0.27, 0.74],
        [2.62, 2.92, 0.2, 0.92],
        [1.55, 2.7, 0.16, 1.08],
      ] as const) {
        const bat = new THREE.Mesh(batGeometry, batMaterial);
        bat.position.set(baseX, baseY, -1.9);
        bat.scale.setScalar(scale);
        scene.add(bat);
        bats.push({ mesh: bat, baseX, baseY, speed });
      }

      const fogCanvas = document.createElement("canvas");
      fogCanvas.width = 128;
      fogCanvas.height = 64;
      const fogContext = fogCanvas.getContext("2d");
      if (fogContext) {
        const gradient = fogContext.createRadialGradient(64, 32, 2, 64, 32, 62);
        gradient.addColorStop(0, "rgba(180, 112, 255, .28)");
        gradient.addColorStop(0.42, "rgba(116, 65, 160, .15)");
        gradient.addColorStop(1, "rgba(30, 10, 42, 0)");
        fogContext.fillStyle = gradient;
        fogContext.fillRect(0, 0, 128, 64);
      }
      const fogTexture = new THREE.CanvasTexture(fogCanvas);
      const fogSprites: Sprite[] = [];
      for (const [x, y, z, scale] of [
        [-2.6, -0.55, 1.15, 3.6],
        [1.8, -0.63, 0.8, 4.1],
        [0.1, -0.75, -0.25, 4.8],
      ] as const) {
        const fog = new THREE.Sprite(
          new THREE.SpriteMaterial({
            map: fogTexture,
            transparent: true,
            depthWrite: false,
            opacity: 0.58,
          }),
        );
        fog.position.set(x, y, z);
        fog.scale.set(scale, scale * 0.42, 1);
        scene.add(fog);
        fogSprites.push(fog);
      }

      function addBranch(parent: Group, from: Vector3, to: Vector3, radius: number) {
        const direction = new THREE.Vector3().subVectors(to, from);
        const branch = new THREE.Mesh(
          new THREE.CylinderGeometry(radius * 0.7, radius, direction.length(), 7),
          new THREE.MeshStandardMaterial({ color: 0x100713, roughness: 1 }),
        );
        branch.position.copy(from).add(to).multiplyScalar(0.5);
        branch.quaternion.setFromUnitVectors(
          new THREE.Vector3(0, 1, 0),
          direction.clone().normalize(),
        );
        branch.castShadow = true;
        parent.add(branch);
      }

      function createTree(x: number, z: number, scale: number, mirrored = false) {
        const tree = new THREE.Group();
        addBranch(tree, new THREE.Vector3(0, -1.2, 0), new THREE.Vector3(0, 1.75, 0), 0.18);
        const side = mirrored ? -1 : 1;
        addBranch(
          tree,
          new THREE.Vector3(0, 0.35, 0),
          new THREE.Vector3(0.95 * side, 1.45, 0.1),
          0.11,
        );
        addBranch(
          tree,
          new THREE.Vector3(0, 0.72, 0),
          new THREE.Vector3(-0.78 * side, 1.72, -0.05),
          0.1,
        );
        addBranch(
          tree,
          new THREE.Vector3(0.92 * side, 1.42, 0.1),
          new THREE.Vector3(1.28 * side, 1.9, 0),
          0.055,
        );
        addBranch(
          tree,
          new THREE.Vector3(-0.76 * side, 1.68, -0.05),
          new THREE.Vector3(-1.03 * side, 2.08, 0.06),
          0.05,
        );
        addBranch(
          tree,
          new THREE.Vector3(0, 1.18, 0),
          new THREE.Vector3(0.55 * side, 2.18, -0.04),
          0.072,
        );
        addBranch(
          tree,
          new THREE.Vector3(0.52 * side, 2.12, -0.04),
          new THREE.Vector3(0.2 * side, 2.52, 0.03),
          0.038,
        );
        addBranch(
          tree,
          new THREE.Vector3(-1.01 * side, 2.04, 0.06),
          new THREE.Vector3(-1.34 * side, 2.28, 0.02),
          0.032,
        );
        tree.position.set(x, -0.05, z);
        tree.scale.setScalar(scale);
        tree.rotation.z = side * 0.07;
        scene.add(tree);
        return tree;
      }

      const leftTree = createTree(-4.35, -0.65, 1.28);
      const rightTree = createTree(4.35, -0.85, 1.2, true);
      createTree(-1.65, -3.25, 0.72, true);
      createTree(2.62, -3.45, 0.78);

      function createGrave(x: number, z: number, rotation: number, scale = 1) {
        const grave = new THREE.Group();
        const stone = new THREE.MeshStandardMaterial({
          color: 0x392342,
          roughness: 0.95,
        });
        const body = new THREE.Mesh(new THREE.BoxGeometry(0.62, 0.86, 0.18), stone);
        body.position.y = -0.36;
        body.castShadow = true;
        grave.add(body);
        const top = new THREE.Mesh(new THREE.SphereGeometry(0.31, 18, 10), stone);
        top.scale.set(1, 0.72, 0.3);
        top.position.y = 0.05;
        top.castShadow = true;
        grave.add(top);
        const crossMaterial = new THREE.MeshStandardMaterial({ color: 0x1a0d21 });
        const crossVertical = new THREE.Mesh(
          new THREE.BoxGeometry(0.08, 0.32, 0.025),
          crossMaterial,
        );
        crossVertical.position.set(0, -0.24, 0.105);
        grave.add(crossVertical);
        const crossHorizontal = new THREE.Mesh(
          new THREE.BoxGeometry(0.25, 0.07, 0.025),
          crossMaterial,
        );
        crossHorizontal.position.set(0, -0.2, 0.11);
        grave.add(crossHorizontal);
        grave.position.set(x, -0.55, z);
        grave.rotation.y = rotation;
        grave.rotation.z = rotation * 0.12;
        grave.scale.setScalar(scale);
        scene.add(grave);
      }

      createGrave(-2.28, 0.25, 0.22, 0.9);
      createGrave(2.38, -0.18, -0.2, 0.82);
      createGrave(-3.05, -1.42, 0.36, 0.62);
      createGrave(3.7, -1.18, -0.28, 0.66);
      createGrave(4.55, -2.05, 0.18, 0.48);

      function createPumpkin(x: number, z: number, scale: number) {
        const pumpkin = new THREE.Group();
        const orange = new THREE.MeshStandardMaterial({
          color: 0xf36f16,
          emissive: 0x6f1600,
          emissiveIntensity: 0.55,
          roughness: 0.66,
        });
        for (let index = -2; index <= 2; index += 1) {
          const segment = new THREE.Mesh(new THREE.SphereGeometry(0.33, 20, 14), orange);
          segment.scale.set(0.72, 1, 0.82);
          segment.position.x = index * 0.15;
          segment.castShadow = true;
          pumpkin.add(segment);
        }
        const stem = new THREE.Mesh(
          new THREE.CylinderGeometry(0.055, 0.075, 0.26, 7),
          new THREE.MeshStandardMaterial({ color: 0x342313 }),
        );
        stem.position.y = 0.38;
        stem.rotation.z = 0.16;
        pumpkin.add(stem);
        const faceMaterial = new THREE.MeshBasicMaterial({ color: 0x210722 });
        for (const eyeX of [-0.16, 0.16]) {
          const eye = new THREE.Mesh(new THREE.TetrahedronGeometry(0.105, 0), faceMaterial);
          eye.position.set(eyeX, 0.08, 0.29);
          eye.rotation.z = Math.PI / 4;
          pumpkin.add(eye);
        }
        const mouth = new THREE.Mesh(new THREE.BoxGeometry(0.3, 0.055, 0.03), faceMaterial);
        mouth.position.set(0, -0.13, 0.315);
        pumpkin.add(mouth);
        const glow = new THREE.PointLight(0xff6b12, 4, 3, 2);
        glow.position.set(0, 0.1, 0.5);
        pumpkin.add(glow);
        pumpkin.position.set(x, -0.72, z);
        pumpkin.scale.setScalar(scale);
        scene.add(pumpkin);
        return { pumpkin, glow };
      }

      const pumpkins = [
        createPumpkin(-4.15, 0.82, 0.7),
        createPumpkin(-3.25, 1.2, 0.94),
        createPumpkin(3.28, 1.02, 0.82),
        createPumpkin(4.22, 0.58, 0.64),
      ];

      const particleGeometry = new THREE.BufferGeometry();
      const particlePositions = new Float32Array(42 * 3);
      for (let index = 0; index < 42; index += 1) {
        particlePositions[index * 3] = (Math.random() - 0.5) * 8;
        particlePositions[index * 3 + 1] = Math.random() * 3.8 - 0.55;
        particlePositions[index * 3 + 2] = Math.random() * 4 - 1.7;
      }
      particleGeometry.setAttribute("position", new THREE.BufferAttribute(particlePositions, 3));
      const particles = new THREE.Points(
        particleGeometry,
        new THREE.PointsMaterial({
          color: 0xc27cff,
          size: 0.035,
          transparent: true,
          opacity: 0.58,
          depthWrite: false,
        }),
      );
      scene.add(particles);

      function resize() {
        const width = Math.max(1, host.clientWidth);
        const height = Math.max(1, host.clientHeight);
        renderer.setSize(width, height, false);
        camera.aspect = width / height;
        camera.updateProjectionMatrix();
        renderer.render(scene, camera);
      }

      const observer = new ResizeObserver(resize);
      observer.observe(host);
      resize();
      const clock = new THREE.Clock();
      let frame = 0;
      function render() {
        if (disposed) return;
        frame = requestAnimationFrame(render);
        if (animated) {
          const elapsed = clock.getElapsedTime();
          particles.rotation.y = elapsed * 0.025;
          leftTree.rotation.z = 0.07 + Math.sin(elapsed * 0.42) * 0.012;
          rightTree.rotation.z = -0.07 - Math.sin(elapsed * 0.38) * 0.011;
          pumpkins.forEach(({ pumpkin, glow }, index) => {
            glow.intensity = 3.2 + Math.sin(elapsed * (1.7 + index * 0.13) + index) * 0.62;
            pumpkin.rotation.y = Math.sin(elapsed * 0.32 + index) * 0.04;
          });
          windowMaterial.opacity = 0.72 + Math.sin(elapsed * 1.7) * 0.12;
          bats.forEach(({ mesh, baseX, baseY, speed }, index) => {
            mesh.position.x = baseX + Math.sin(elapsed * speed + index) * 0.18;
            mesh.position.y = baseY + Math.sin(elapsed * speed * 1.7 + index) * 0.08;
            mesh.rotation.z = Math.sin(elapsed * speed * 2.4 + index) * 0.12;
          });
          fogSprites.forEach((fog, index) => {
            fog.position.x += Math.sin(elapsed * 0.12 + index * 1.8) * 0.0007;
            fog.material.opacity = 0.45 + Math.sin(elapsed * 0.3 + index) * 0.11;
          });
        }
        renderer.render(scene, camera);
      }
      render();

      cleanup = () => {
        observer.disconnect();
        cancelAnimationFrame(frame);
        scene.traverse((object) => {
          if (
            !(object instanceof THREE.Mesh) &&
            !(object instanceof THREE.Points) &&
            !(object instanceof THREE.Sprite)
          )
            return;
          object.geometry.dispose();
          const materials = Array.isArray(object.material) ? object.material : [object.material];
          materials.forEach((material) => {
            if ("map" in material && material.map) material.map.dispose();
            material.dispose();
          });
        });
        renderer.dispose();
        renderer.forceContextLoss();
      };
    });

    return () => {
      disposed = true;
      cleanup();
    };
  });
</script>

<div bind:this={host} class="halloween-scene size-full" aria-hidden="true">
  <canvas bind:this={canvas} class="size-full"></canvas>
</div>

<style>
  .halloween-scene {
    overflow: hidden;
    background:
      radial-gradient(circle at 55% 24%, rgb(175 211 255 / 0.2), transparent 23%),
      radial-gradient(circle at 22% 52%, rgb(255 108 24 / 0.08), transparent 20%),
      radial-gradient(ellipse at 50% 82%, rgb(125 64 168 / 0.2), transparent 62%),
      linear-gradient(180deg, rgb(12 24 43 / 0.72) 0%, rgb(11 13 25 / 0.65) 54%, #060407 100%);
    -webkit-mask-image:
      linear-gradient(
        90deg,
        transparent 0%,
        rgb(0 0 0 / 0.16) 4%,
        rgb(0 0 0 / 0.72) 11%,
        #000 20%,
        #000 80%,
        rgb(0 0 0 / 0.72) 89%,
        rgb(0 0 0 / 0.16) 96%,
        transparent 100%
      ),
      linear-gradient(
        180deg,
        transparent 0%,
        rgb(0 0 0 / 0.68) 8%,
        #000 18%,
        #000 78%,
        rgb(0 0 0 / 0.64) 89%,
        transparent 100%
      );
    -webkit-mask-composite: source-in;
    mask-image:
      linear-gradient(
        90deg,
        transparent 0%,
        rgb(0 0 0 / 0.16) 4%,
        rgb(0 0 0 / 0.72) 11%,
        #000 20%,
        #000 80%,
        rgb(0 0 0 / 0.72) 89%,
        rgb(0 0 0 / 0.16) 96%,
        transparent 100%
      ),
      linear-gradient(
        180deg,
        transparent 0%,
        rgb(0 0 0 / 0.68) 8%,
        #000 18%,
        #000 78%,
        rgb(0 0 0 / 0.64) 89%,
        transparent 100%
      );
    mask-composite: intersect;
  }

  canvas {
    display: block;
    filter: saturate(0.96) contrast(1.04) drop-shadow(0 18px 28px rgb(0 0 0 / 0.42));
  }
</style>
