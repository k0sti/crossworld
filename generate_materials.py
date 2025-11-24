import json

with open('assets/materials.json', 'r') as f:
    data = json.load(f)

print("pub const MATERIAL_REGISTRY: [Material; 128] = [")
for i in range(128):
    mat = next((m for m in data['materials'] if m['index'] == i), None)
    if mat:
        # Parse hex color #AARRGGBB or #RRGGBB
        hex_color = mat['color'].lstrip('#')
        if len(hex_color) == 8:
            # ARGB format in JSON? The file shows #80FFFFFF etc.
            # Let's assume AARRGGBB based on the values.
            # But wait, index 0 is #00000000 (transparent).
            # Index 2 glass is #80FFFFFF.
            # Rust glam::Vec3 is RGB. We might drop alpha or store it?
            # The renderer uses Vec3 for color.
            # Let's extract RGB.
            r = int(hex_color[2:4], 16) / 255.0
            g = int(hex_color[4:6], 16) / 255.0
            b = int(hex_color[6:8], 16) / 255.0
        elif len(hex_color) == 6:
            r = int(hex_color[0:2], 16) / 255.0
            g = int(hex_color[2:4], 16) / 255.0
            b = int(hex_color[4:6], 16) / 255.0
        else:
            r, g, b = 0.0, 0.0, 0.0
        
        print(f"    Material {{ index: {i}, id: \"{mat['id']}\", color: Vec3::new({r:.3f}, {g:.3f}, {b:.3f}) }},")
    else:
        print(f"    Material {{ index: {i}, id: \"unknown\", color: Vec3::ZERO }},")
print("];")
