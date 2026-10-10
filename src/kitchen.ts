import type { Lot } from "./inventory-list";
import { renderInventoryList } from "./inventory-list";
import * as THREE from "three";

export function zoneFromHitName(name: string): string | null {
  return ["zone-fridge", "zone-freezer", "zone-pantry"].includes(name)
    ? name.slice(5)
    : null;
}

export function renderKitchen(
  root: HTMLElement,
  lots: Lot[],
  onZone: (zone: string) => void,
  webglAvailable = true
): "3d" | "list" {
  root.replaceChildren();
  const fallback = () => {
    const note = document.createElement("p");
    note.textContent = "3D view unavailable. Use this inventory list instead.";
    root.append(note);
    const list = document.createElement("div");
    root.append(list);
    renderInventoryList(list, lots, (_id, zone) => onZone(zone));
    return "list" as const;
  };
  if (!webglAvailable) return fallback();

  let renderer: THREE.WebGLRenderer;
  try {
    renderer = new THREE.WebGLRenderer({ antialias: true, alpha: true });
  } catch {
    return fallback();
  }
  const width = Math.max(root.clientWidth, 500);
  const height = 310;
  renderer.setSize(width, height);
  renderer.setPixelRatio(Math.min(window.devicePixelRatio || 1, 2));
  const scene = new THREE.Scene();
  scene.background = new THREE.Color("#edf3ef");
  const camera = new THREE.PerspectiveCamera(42, width / height, 0.1, 100);
  camera.position.set(0, 5.5, 10);
  camera.lookAt(0, 1.2, 0);
  scene.add(new THREE.AmbientLight("#ffffff", 1.7));
  const light = new THREE.DirectionalLight("#ffffff", 1.8);
  light.position.set(-3, 8, 5);
  scene.add(light);
  const floor = new THREE.Mesh(
    new THREE.BoxGeometry(9, 0.12, 4.3),
    new THREE.MeshStandardMaterial({ color: "#d2bca1" })
  );
  floor.position.y = -0.1;
  scene.add(floor);
  const objects: THREE.Object3D[] = [];
  const cabinets: Array<[string, number, number, number, string]> = [
    ["fridge", -2.7, 1.55, 2.9, "#d9e8e8"],
    ["freezer", 0, 1.1, 2.0, "#9acdd5"],
    ["pantry", 2.7, 1.35, 2.5, "#ba865b"]
  ];
  for (const [zone, x, y, heightOfBox, color] of cabinets) {
    const box = new THREE.Mesh(
      new THREE.BoxGeometry(2.0, heightOfBox, 1.8),
      new THREE.MeshStandardMaterial({ color, roughness: 0.8 })
    );
    box.name = `zone-${zone}`;
    box.position.set(x, y, 0);
    scene.add(box);
    objects.push(box);
  }
  renderer.render(scene, camera);
  renderer.domElement.setAttribute("aria-hidden", "true");
  renderer.domElement.addEventListener("pointerdown", (event) => {
    const rect = renderer.domElement.getBoundingClientRect();
    const pointer = new THREE.Vector2(
      ((event.clientX - rect.left) / rect.width) * 2 - 1,
      -((event.clientY - rect.top) / rect.height) * 2 + 1
    );
    const ray = new THREE.Raycaster();
    ray.setFromCamera(pointer, camera);
    const hit = ray.intersectObjects(objects)[0];
    const zone = hit && zoneFromHitName(hit.object.name);
    if (zone) onZone(zone);
  });
  root.append(renderer.domElement);
  const controls = document.createElement("div");
  controls.className = "kitchen-zones";
  for (const zone of ["fridge", "freezer", "pantry"]) {
    const button = document.createElement("button");
    button.type = "button";
    button.textContent = `${zone[0].toUpperCase()}${zone.slice(1)} (${lots.filter((l) => l.zone === zone && Number(l.quantity) > 0).length})`;
    button.addEventListener("click", () => onZone(zone));
    controls.append(button);
  }
  root.append(controls);
  return "3d";
}
