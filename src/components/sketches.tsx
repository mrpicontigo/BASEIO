// Six hand-drawn style line sketches used as a quiet animated backdrop.
// Every shape uses pathLength=1 so the CSS "draw-in" animation works uniformly.
import type { ReactNode } from "react";

const P = (d: string) => <path className="stroke-draw" pathLength={1} d={d} />;

export interface Sketch { name: string; color: "cyan" | "yellow"; body: ReactNode }

export const SKETCHES: Sketch[] = [
  {
    name: "rocket",
    color: "cyan",
    body: (
      <>
        {P("M100 26 C122 48 130 80 127 120 L73 120 C70 80 78 48 100 26 Z")}
        <circle className="stroke-draw" pathLength={1} cx="100" cy="74" r="11" />
        <circle className="stroke-draw" pathLength={1} cx="100" cy="74" r="6" />
        {P("M74 98 L54 132 L75 125")}
        {P("M126 98 L146 132 L125 125")}
        {P("M86 120 L83 131 L117 131 L114 120")}
        {P("M89 134 C91 150 100 164 100 176 C100 164 109 150 111 134")}
        {P("M95 136 C97 146 100 152 100 158 C100 152 103 146 105 136")}
        {P("M100 26 L100 40")}
        {P("M38 40 L38 50 M33 45 L43 45")}
        {P("M162 58 L162 66 M158 62 L166 62")}
        {P("M152 158 L152 166 M148 162 L156 162")}
        {P("M44 150 L44 156 M41 153 L47 153")}
        {P("M60 170 L70 156 M136 170 L130 160")}
      </>
    ),
  },
  {
    name: "goat",
    color: "yellow",
    body: (
      <>
        {P("M58 100 C58 86 72 80 92 80 L130 80 C144 80 150 90 150 102 C150 115 140 122 126 122 L80 122 C67 122 58 112 58 100 Z")}
        {P("M138 84 C144 76 146 72 150 70")}
        {P("M148 70 L156 52 C160 46 170 46 174 52 L181 64 C182 68 179 71 175 71 L160 75 Z")}
        {P("M160 50 C155 37 145 31 136 34")}
        {P("M167 48 C167 35 160 27 151 27")}
        {P("M157 57 L145 62")}
        {P("M176 71 L178 84 L171 75")}
        <circle cx="167" cy="58" r="1.6" fill="currentColor" stroke="none" />
        {P("M72 120 L69 152 M86 122 L88 152 M122 122 L119 152 M136 119 L140 152")}
        {P("M64 152 L74 152 M83 152 L93 152 M114 152 L124 152 M135 152 L145 152")}
        {P("M60 94 L49 85 L55 97")}
        {P("M84 92 C92 96 104 96 112 92")}
        {P("M30 156 L180 156")}
        {P("M40 156 L44 148 L48 156 M160 156 L163 150 L166 156")}
      </>
    ),
  },
  {
    name: "boat",
    color: "cyan",
    body: (
      <>
        {P("M48 128 L152 128 L137 148 L63 148 Z")}
        {P("M100 128 L100 38")}
        {P("M103 44 L103 122 L148 122 Z")}
        {P("M97 52 L97 122 L60 122 Z")}
        {P("M100 38 L113 42 L100 47")}
        {P("M72 138 L80 138 M88 138 L96 138")}
        {P("M26 156 Q36 149 46 156 T66 156 T86 156 T106 156 T126 156 T146 156 T166 156 T186 156")}
        {P("M40 168 Q50 161 60 168 T80 168 T100 168 T120 168 T140 168 T160 168")}
        <circle className="stroke-draw" pathLength={1} cx="160" cy="48" r="11" />
        {P("M36 62 q6 -7 12 0 q6 -7 12 0")}
        {P("M56 44 q4 -5 8 0 q4 -5 8 0")}
      </>
    ),
  },
  {
    name: "mountains",
    color: "yellow",
    body: (
      <>
        {P("M14 160 L78 66 L110 112 L132 86 L186 160")}
        {P("M66 84 L78 66 L91 84 L85 80 L78 88 L72 80 Z")}
        {P("M123 98 L132 86 L141 98 L136 95 L132 100 L127 95 Z")}
        {P("M94 160 L112 132 L126 146")}
        <circle className="stroke-draw" pathLength={1} cx="150" cy="46" r="13" />
        {P("M150 24 L150 28 M150 64 L150 68 M128 46 L132 46 M168 46 L172 46")}
        {P("M32 62 q4 -10 14 -6 q6 -9 15 0 q9 0 9 8 L32 64")}
        {P("M8 160 L192 160")}
        {P("M36 160 L42 145 L48 160 M42 145 L42 160")}
        {P("M154 160 L160 142 L166 160 M160 142 L160 160")}
        {P("M168 160 L172 150 L176 160")}
      </>
    ),
  },
  {
    name: "dog",
    color: "cyan",
    body: (
      <>
        {P("M80 152 C70 132 73 106 91 97 C107 90 124 100 126 120 C128 137 124 149 118 152 Z")}
        {P("M88 96 C84 80 92 63 108 63 C124 63 133 78 129 92 C125 101 112 105 100 103 C94 103 90 100 88 96 Z")}
        {P("M93 71 C82 69 77 84 84 95 C88 90 91 82 93 76")}
        {P("M124 69 C135 69 137 84 131 93 C127 89 125 81 124 74")}
        <circle cx="102" cy="80" r="2.2" fill="currentColor" stroke="none" />
        <circle cx="117" cy="80" r="2.2" fill="currentColor" stroke="none" />
        {P("M105 88 Q109 93 113 88 Q109 85 105 88 Z")}
        {P("M109 92 L109 96 M103 96 Q109 101 115 96")}
        {P("M92 102 Q108 110 125 99")}
        <circle className="stroke-draw" pathLength={1} cx="108" cy="108" r="3.5" />
        {P("M96 122 L94 152 M108 122 L110 152")}
        {P("M88 152 L100 152 M104 152 L117 152")}
        {P("M124 142 C141 140 147 125 142 116")}
        {P("M60 156 L160 156")}
        <circle className="stroke-draw" pathLength={1} cx="150" cy="146" r="8" />
        {P("M144 141 C148 145 152 145 156 141 M144 151 C148 147 152 147 156 151")}
      </>
    ),
  },
  {
    name: "books",
    color: "yellow",
    body: (
      <>
        <rect className="stroke-draw" pathLength={1} x="48" y="132" width="104" height="18" rx="2" />
        {P("M57 132 L57 150 M142 136 L149 136 M142 141 L149 141 M142 146 L149 146")}
        <g transform="rotate(-3 103 122)">
          <rect className="stroke-draw" pathLength={1} x="57" y="113" width="92" height="18" rx="2" />
          {P("M66 113 L66 131 M80 120 L110 120")}
        </g>
        <rect className="stroke-draw" pathLength={1} x="44" y="95" width="98" height="18" rx="2" />
        {P("M134 95 L134 113 M54 102 L96 102 M54 107 L80 107")}
        <g transform="rotate(4 100 85)">
          <rect className="stroke-draw" pathLength={1} x="62" y="76" width="78" height="18" rx="2" />
          {P("M70 76 L70 94")}
          {P("M118 94 L118 104 L121 101 L124 104 L124 94")}
        </g>
        {P("M100 58 C89 53 84 69 91 74 C96 78 104 78 109 74 C116 69 111 53 100 58 Z")}
        {P("M100 58 L102 50 M102 53 C107 49 112 51 113 54 C108 56 104 55 102 53")}
        {P("M30 152 L170 152")}
      </>
    ),
  },
];
