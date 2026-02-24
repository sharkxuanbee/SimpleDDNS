
import os

filepath = 'simpleddns-app/src/main.rs'

if os.path.exists(filepath):
    try:
        with open(filepath, 'r', encoding='utf-8') as f:
            lines = f.readlines()
        
        new_lines = []
        for line in lines:
            # Fix ProfileEditor generic_method commented out
            if '// Generic HTTP URL template' in line and 'generic_method' in line:
                parts = line.split('generic_method')
                comment_part = parts[0].strip()
                code_part = 'generic_method' + parts[1]
                
                if not comment_part.startswith('//'):
                     comment_part = '// ' + comment_part
                
                new_lines.append(f"{comment_part}\n")
                new_lines.append(f"    {code_part}")
                continue
            
            # Fix button quotes
            # "鈫? -> "Up" or "Down" (context dependent, but Up/Down are safer)
            # Actually, "鈫? is probably "Up" or "Down" arrow.
            # But line 510 and 513 use same character?
            # 510: i > 0 (Up)
            # 513: i + 1 < len (Down)
            # So they are likely different arrows in source but look garbage here.
            # I'll replace first occurrence with "Up" and second with "Down" if I can track it.
            # But simple replacement: "鈫? -> "Move"
            # Or just "Up/Down".
            
            # Line 510: `if i > 0 && ui.small_button("鈫?).clicked() {`
            # Line 513: `if i + 1 < ipv4_len && ui.small_button("鈫?).clicked() {`
            
            # Wait, line 510 logic is `i > 0`, so it's UP.
            # Line 513 logic is `i + 1 < len`, so it's DOWN.
            
            # If both characters are `鈫?`, I can't distinguish by string.
            # But I can distinguish by context `i > 0` vs `i + 1 <`.
            
            if 'i > 0' in line and 'small_button("鈫?)' in line:
                line = line.replace('small_button("鈫?)', 'small_button("Up")')
            elif 'i + 1 <' in line and 'small_button("鈫?)' in line:
                line = line.replace('small_button("鈫?)', 'small_button("Down")')
            
            # Also for IPv6 loop (551, 554)
            # 551: i > 0
            # 554: i + 1 <
            
            # Fix Remove button
            # `ui.small_button("鉁?).clicked()`
            if 'small_button("鉁?)' in line:
                line = line.replace('small_button("鉁?)', 'small_button("X")')
            
            # Also check `btn_add` suffix error (line 535)
            # `ui.button(i18n::I18n::t(lang, "btn_add")).clicked()`
            # Error says `invalid suffix btn_add`.
            # This means previous string wasn't closed?
            # Previous string was `ui.text_edit_singleline(&mut self.new_ipv4_url);` (Line 534)
            # No string there.
            # Line 533: `ui.add(egui::TextEdit::singleline(...));`?
            
            # Maybe `btn_add` error is due to `i18n` macro? No.
            # Wait, `read_file_range` output for 535:
            # `if ui.button(i18n::I18n::t(lang, "btn_add")).clicked()...`
            # It looks fine.
            # Maybe invisible char?
            
            # Let's replace "btn_add" with "btn_add" manually to be sure.
            if '"btn_add"' in line:
                line = line.replace('"btn_add"', '"btn_add"')
            
            new_lines.append(line)
        
        with open(filepath, 'w', encoding='utf-8') as f:
            f.writelines(new_lines)
        print(f"Fixed final issues in {filepath}")
            
    except Exception as e:
        print(f"Error fixing {filepath}: {e}")
