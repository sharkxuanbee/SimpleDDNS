
import os
import re

filepath = 'simpleddns-app/src/main.rs'

if os.path.exists(filepath):
    try:
        with open(filepath, 'r', encoding='utf-8') as f:
            lines = f.readlines()
        
        start_idx = -1
        for i, line in enumerate(lines):
            if 'match self.active_tab' in line:
                start_idx = i
                print(f"Found start at line {i+1}: {line.strip()}")
                break
        
        if start_idx != -1:
            end_idx = -1
            for j in range(start_idx, len(lines)):
                if '});' in lines[j]:
                    end_idx = j
                    print(f"Found end at line {j+1}: {lines[j].strip()}")
                    break
            
            if end_idx != -1:
                # Construct new block
                # Preserve indentation from start line
                indent = lines[start_idx].split('egui')[0] # approximate indentation
                if not indent.strip() == '': # If line starts with egui
                     indent = '        ' # default
                
                # Check actual indentation
                match = re.match(r'^(\s*)', lines[start_idx])
                if match:
                    indent = match.group(1)
                
                new_block = [
                    f'{indent}egui::CentralPanel::default().show(ctx, |ui| match self.active_tab {{\n',
                    f'{indent}    0 => self.render_profiles(ui, lang),\n',
                    f'{indent}    1 => self.render_settings(ui, lang),\n',
                    f'{indent}    _ => {{}}\n',
                    f'{indent}}});\n' # } closes match, } closes closure, ); closes show
                ]
                
                # Replace
                lines[start_idx : end_idx+1] = new_block
                
                with open(filepath, 'w', encoding='utf-8') as f:
                    f.writelines(lines)
                print(f"Cleaned match block in {filepath}")
            else:
                print("Could not find end of match block")
        else:
            print("Could not find start of match block")
            
    except Exception as e:
        import traceback
        traceback.print_exc()
        print(f"Error fixing {filepath}: {e}")
else:
    print(f"File not found: {filepath}")
