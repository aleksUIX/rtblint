#!/usr/bin/env python3
"""
Convert preprint-vastlint.md to preprint-vastlint.pdf using fpdf2.
Clean academic-style PDF, no system deps beyond Python + fpdf2.
"""

import re
from pathlib import Path
from fpdf import FPDF

SRC = Path(__file__).parent / "preprint-openrtb-wild.md"
OUT = Path(__file__).parent / "preprint-openrtb-wild.pdf"

# ---- fonts ----
BODY_SIZE = 10.5
SMALL_SIZE = 9
H1_SIZE = 15
H2_SIZE = 13
H3_SIZE = 11.5
LINE_H = 5.2


class Paper(FPDF):
    def __init__(self):
        super().__init__(format="letter")
        self.set_auto_page_break(auto=True, margin=25)

    def footer(self):
        self.set_y(-15)
        self.set_font("Helvetica", "", 9)
        self.set_text_color(120, 120, 120)
        self.cell(0, 10, str(self.page_no()), align="C")

    # ---- helpers ----

    def h1(self, text):
        self.set_font("Helvetica", "B", H1_SIZE)
        self.set_text_color(0)
        self.ln(4)
        self.multi_cell(0, 8, text, align="C")
        self.ln(3)

    def h2(self, text):
        self.set_font("Helvetica", "B", H2_SIZE)
        self.set_text_color(0)
        self.ln(5)
        self.multi_cell(0, 7, text)
        y = self.get_y()
        self.line(self.l_margin, y, self.w - self.r_margin, y)
        self.ln(3)

    def h3(self, text):
        self.set_font("Helvetica", "B", H3_SIZE)
        self.set_text_color(0)
        self.ln(3)
        self.multi_cell(0, 6, text)
        self.ln(2)

    def para(self, text):
        self.set_font("Helvetica", "", BODY_SIZE)
        self.set_text_color(30)
        self.multi_cell(0, LINE_H, text)
        self.ln(2)

    def centered(self, text, size=BODY_SIZE, bold=False, italic=False, color=0):
        style = ""
        if bold: style += "B"
        if italic: style += "I"
        self.set_font("Helvetica", style, size)
        self.set_text_color(color)
        self.cell(0, 6, text, align="C", ln=True)

    def bullet(self, text):
        self.set_font("Helvetica", "", BODY_SIZE)
        self.set_text_color(30)
        indent = 8
        self.set_x(self.l_margin + indent)
        w = self.w - self.l_margin - self.r_margin - indent
        # bullet char
        self.cell(5, LINE_H, "-")
        self.multi_cell(w - 5, LINE_H, text)
        self.ln(1)

    def numbered(self, n, text):
        self.set_font("Helvetica", "", BODY_SIZE)
        self.set_text_color(30)
        indent = 8
        self.set_x(self.l_margin + indent)
        w = self.w - self.l_margin - self.r_margin - indent
        self.cell(7, LINE_H, f"{n}.")
        self.multi_cell(w - 7, LINE_H, text)
        self.ln(1)

    def table(self, headers, rows, number=None):
        # Numbered label above the table, so in-text "Table 3" references resolve.
        if number is not None:
            self.ln(1)
            self.set_font("Helvetica", "B", 8)
            self.set_text_color(90, 90, 90)
            self.cell(0, 4.5, f"Table {number}", ln=True)
            self.set_text_color(0, 0, 0)
        table_w = self.w - self.l_margin - self.r_margin
        n = len(headers)
        # proportional widths
        all_rows = [headers] + rows
        maxl = [0] * n
        for r in all_rows:
            for j in range(min(n, len(r))):
                maxl[j] = max(maxl[j], len(r[j]))
        total = sum(maxl) or 1
        cw = [(m / total) * table_w for m in maxl]
        rh = 6

        # header
        self.set_font("Helvetica", "B", SMALL_SIZE)
        self.set_fill_color(235, 235, 235)
        self.set_text_color(0)
        for j, h in enumerate(headers):
            self.cell(cw[j], rh, _latin1(h.strip()[:40]), border=1, fill=True)
        self.ln()

        # body
        self.set_font("Helvetica", "", SMALL_SIZE)
        self.set_text_color(30)
        for ri, row in enumerate(rows):
            if self.get_y() + rh > self.h - 25:
                self.add_page()
            fill = ri % 2 == 1
            if fill:
                self.set_fill_color(248, 248, 248)
            for j in range(n):
                cell_text = _latin1(row[j].strip()) if j < len(row) else ""
                self.cell(cw[j], rh, cell_text[:60], border=1, fill=fill)
            self.ln()
        self.ln(3)

    def figure(self, path, caption=""):
        if not path.exists():
            return
        avail = self.w - self.l_margin - self.r_margin
        img_w = avail * 0.82
        if self.get_y() + 75 > self.h - 25:
            self.add_page()
        x = self.l_margin + (avail - img_w) / 2
        self.image(str(path), x=x, w=img_w)
        self.ln(2)
        if caption:
            self.set_font("Helvetica", "I", 9)
            self.set_text_color(80)
            self.multi_cell(0, 4.5, caption, align="C")
            self.ln(3)


def clean(text):
    """Strip markdown inline formatting and normalize unicode for latin-1."""
    text = re.sub(r'\*\*(.+?)\*\*', r'\1', text)
    text = re.sub(r'\*(.+?)\*', r'\1', text)
    text = re.sub(r'`(.+?)`', r'\1', text)
    text = re.sub(r'\[([^\]]+)\]\([^)]+\)', r'\1', text)
    # normalize unicode chars to latin-1 safe equivalents
    text = text.replace('\u2014', '--')   # em dash
    text = text.replace('\u2013', '-')    # en dash
    text = text.replace('\u201c', '"').replace('\u201d', '"')
    text = text.replace('\u2018', "'").replace('\u2019', "'")
    text = text.replace('\u2022', '-')    # bullet
    text = text.replace('\u00b5', 'u')    # micro sign -> u
    text = text.replace('\u00a7', 'S')    # section sign
    text = text.replace('\xb7', '.')      # middle dot
    return text


def _latin1(text):
    """Normalize any unicode to latin-1 safe chars (for table cells etc)."""
    text = text.replace('\u2014', '--')
    text = text.replace('\u2013', '-')
    text = text.replace('\u00b5', 'u')    # µ -> u
    text = text.replace('\u00a7', 'S')
    text = text.replace('\u2022', '-')
    text = text.replace('\u201c', '"').replace('\u201d', '"')
    text = text.replace('\u2018', "'").replace('\u2019', "'")
    text = text.replace('\xb7', '.')
    # strip any remaining non-latin1
    text = text.encode('latin-1', errors='replace').decode('latin-1')
    return text


def parse_table(lines, start):
    hdr = [c.strip() for c in lines[start].strip().strip("|").split("|")]
    i = start + 2  # skip separator
    rows = []
    while i < len(lines) and "|" in lines[i] and lines[i].strip().startswith("|"):
        row = [c.strip() for c in lines[i].strip().strip("|").split("|")]
        rows.append(row)
        i += 1
    return hdr, rows, i


def main():
    text = SRC.read_text()
    lines = text.split("\n")
    pdf = Paper()
    pdf.add_page()
    pdf.set_margins(25.4, 25.4, 25.4)

    i = 0
    table_no = [0]  # sequential table numbering; in-text references rely on it
    in_header_block = True  # first few lines are title/author/affil

    while i < len(lines):
        s = lines[i].strip()

        # blank
        if not s:
            i += 1
            continue

        # horizontal rule
        if s == "---":
            in_header_block = False
            i += 1
            continue

        # H1
        if s.startswith("# ") and not s.startswith("## "):
            pdf.h1(clean(s[2:]))
            i += 1
            continue

        # Header block: author, affiliation, date
        if in_header_block:
            if s.startswith("**") and s.endswith("**"):
                pdf.centered(clean(s.strip("*")), size=11, bold=True)
            elif s.startswith("*") and s.endswith("*"):
                pdf.centered(clean(s.strip("*")), size=10, italic=True, color=100)
                pdf.ln(3)
            else:
                pdf.centered(clean(s), size=10, color=80)
            i += 1
            continue

        # H2
        if s.startswith("## "):
            pdf.h2(clean(s[3:]))
            i += 1
            continue

        # H3
        if s.startswith("### "):
            pdf.h3(clean(s[4:]))
            i += 1
            continue

        # Table
        if s.startswith("|") and i + 1 < len(lines) and "---" in lines[i + 1]:
            hdr, rows, end = parse_table(lines, i)
            table_no[0] += 1
            pdf.table(hdr, rows, number=table_no[0])
            i = end
            continue

        # Image
        if s.startswith("!["):
            cap, path = "", None
            m = re.match(r'!\[(.*?)\]\((.+?)\)', s)
            if m:
                cap = m.group(1)
                path = SRC.parent / m.group(2)
            if path is not None:
                pdf.figure(path, cap)
            i += 1
            continue

        # Bullet
        if s.startswith("- "):
            pdf.bullet(clean(s[2:]))
            i += 1
            continue

        # Numbered
        m = re.match(r'^(\d+)\.\s+(.+)', s)
        if m:
            pdf.numbered(m.group(1), clean(m.group(2)))
            i += 1
            continue

        # Regular paragraph — collect continuation
        para = s
        while (i + 1 < len(lines)
               and lines[i+1].strip()
               and not lines[i+1].strip().startswith("#")
               and not lines[i+1].strip().startswith("- ")
               and not lines[i+1].strip().startswith("|")
               and not lines[i+1].strip().startswith("![")
               and not lines[i+1].strip().startswith("---")
               and not re.match(r'^\d+\.', lines[i+1].strip())
               and not (lines[i+1].strip().startswith("*") and lines[i+1].strip().endswith("*"))):
            i += 1
            para += " " + lines[i].strip()

        pdf.para(clean(para))
        i += 1

    pdf.output(str(OUT))
    print(f"Done: {OUT}")
    print(f"Size: {OUT.stat().st_size / 1024:.0f} KB")


if __name__ == "__main__":
    main()
