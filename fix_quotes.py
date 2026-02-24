
import os

filepath = 'simpleddns-app/src/main.rs'

if os.path.exists(filepath):
    try:
        with open(filepath, 'r', encoding='utf-8') as f:
            lines = f.readlines()
        
        for i, line in enumerate(lines):
            if 'None => "' in line and '",' in line:
                # Check if quote is missing
                # simple heuristic: odd number of quotes?
                # But here we see `None => "鈿?,`
                # If `鈿?` is garbage, maybe the quote is gone.
                
                # We'll just replace the whole match line 309 logic with safe strings
                # 309: Some(s) if s.status_message == "OK" => "馃煝", Some(_) => "馃敶", None => "鈿?, };
                
                if 'status_message == "OK"' in line:
                    # Replace with safe version
                    # "OK" -> "OK"
                    # "..." -> "Running"
                    # "..." -> "Unknown"
                    
                    # We can replace the whole line content
                    # But indentation?
                    
                    # Regex might be safer
                    pass
                
        # Better: just read the file and replace the specific bad string pattern
        # "鈿? is likely the culprit
        
        with open(filepath, 'r', encoding='utf-8') as f:
            content = f.read()
        
        # Replace the bad line
        # Use regex to be flexible with whitespace
        # Pattern: Some\(s\) if s\.status_message == "OK" => ".*?",\s*Some\(_\) => ".*?",\s*None => ".*?,
        
        # Actually, let's just replace the specific garbage with proper quotes.
        # The log showed: `None => "鈿?,`
        # It should be `None => "...",`
        
        if 'None => "鈿?,' in content:
            content = content.replace('None => "鈿?,', 'None => "Unknown",')
            print("Fixed None quote")
        elif 'None => "鈿?' in content: # Maybe comma is next line?
             content = content.replace('None => "鈿?', 'None => "Unknown"')
             print("Fixed None quote (variant 2)")
             
        # Also fix the other emojis if they are garbage
        # `Some(s) if s.status_message == "OK" => "馃煝",`
        # `Some(_) => "馃敶",`
        
        if '=> "馃煝",' in content:
            content = content.replace('=> "馃煝",', '=> "OK",')
            print("Fixed OK quote")
            
        if '=> "馃敶",' in content:
            content = content.replace('=> "馃敶",', '=> "Running",')
            print("Fixed Running quote")
            
        with open(filepath, 'w', encoding='utf-8') as f:
            f.write(content)
            
    except Exception as e:
        print(f"Error fixing {filepath}: {e}")
