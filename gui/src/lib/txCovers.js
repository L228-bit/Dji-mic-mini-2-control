export const txCovers = [
  { id: "obsidian-black", name: "墨曜黑", swatch: "#343536", x: 0, y: 0 },
  { id: "glaze-white", name: "云璃白", swatch: "#f2f2f0", x: 1, y: 0 },
  { id: "iris-purple", name: "鸢尾紫", swatch: "#8d58bd", x: 2, y: 0 },
  { id: "starry-blue", name: "星夜蓝", swatch: "#707abc", x: 3, y: 0 },
  { id: "azure-blue", name: "蔚海蓝", swatch: "#19b7dc", x: 4, y: 0 },
  { id: "misty-teal", name: "远山青", swatch: "#10b7ad", x: 0, y: 1 },
  { id: "pine-green", name: "松影绿", swatch: "#2bc38b", x: 1, y: 1 },
  { id: "golden-yellow", name: "金穗黄", swatch: "#e6c300", x: 2, y: 1 },
  { id: "sunrise-orange", name: "朝晖橙", swatch: "#f18a19", x: 3, y: 1 },
  { id: "blush-red", name: "绯樱红", swatch: "#e94b98", x: 4, y: 1 },
];

export const txCover = (id) => txCovers.find((cover) => cover.id === id) ?? txCovers[0];
export const txCoverPosition = (cover) => `${cover.x * 25}% ${cover.y * 100}%`;
