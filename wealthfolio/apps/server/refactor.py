import sys
import glob
import re

def process_file(filepath):
    with open(filepath, 'r', encoding='utf-8') as f:
        lines = f.readlines()

    out_lines = []
    i = 0
    modified = False
    
    while i < len(lines):
        line = lines[i]
        match_derive = re.search(r'^([ \t]*)(#\[derive\((.*?)\)\])', line)
        
        if match_derive and ('Serialize' in match_derive.group(3) or 'Deserialize' in match_derive.group(3)):
            indent = match_derive.group(1)
            derive_str = match_derive.group(2)
            inner = match_derive.group(3)
            
            j = i + 1
            found_struct = False
            struct_indent = ""
            
            while j < min(i + 20, len(lines)):
                lookahead = lines[j]
                
                sm = re.search(r'^([ \t]*)(?:pub(?:\s*\([^)]+\))?\s+)?struct\s+\w+', lookahead)
                if sm:
                    found_struct = True
                    struct_indent = sm.group(1)
                    break
                    
                if re.search(r'\b(?:enum|fn|impl)\b', lookahead):
                    break
                    
                j += 1
                
            if found_struct:
                if 'ts_rs::TS' not in inner and not re.search(r'\bTS\b', inner):
                    new_inner = inner + ", ts_rs::TS"
                    new_line = indent + f"#[derive({new_inner})]\n"
                else:
                    new_line = line
                
                out_lines.append(new_line)
                i += 1
                
                has_ts_export = False
                while i < j:
                    if '#[ts(export)]' in lines[i]:
                        has_ts_export = True
                    out_lines.append(lines[i])
                    i += 1
                    
                if not has_ts_export:
                    out_lines.append(f"{struct_indent}#[ts(export)]\n")
                    
                out_lines.append(lines[i]) # The struct line
                i += 1
                modified = True
                continue
                
        out_lines.append(line)
        i += 1

    if modified:
        with open(filepath, 'w', encoding='utf-8') as f:
            f.writelines(out_lines)
        print(f"Updated {filepath}")

for filepath in glob.glob('/home/astro/wealthfolio/apps/server/src/**/*.rs', recursive=True):
    process_file(filepath)
