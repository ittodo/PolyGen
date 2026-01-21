/**
 * PolyGen Schema Diagram Renderer
 * Parses .poly schema and renders ER-style diagrams with relationships
 */

const PolyDiagram = (function() {
    // Parse schema to extract structure
    function parseSchema(input) {
        const tables = [];
        const enums = [];
        const embeds = [];
        const relationships = [];

        // Remove comments for easier parsing
        const lines = input.split('\n');
        let currentNamespace = '';
        let currentBlock = null;
        let blockType = null;
        let braceDepth = 0;
        let blockContent = [];

        for (const line of lines) {
            const trimmed = line.replace(/\/\/.*$/, '').trim();
            if (!trimmed) continue;

            // Namespace detection
            const nsMatch = trimmed.match(/^namespace\s+([\w.]+)\s*\{?/);
            if (nsMatch) {
                currentNamespace = nsMatch[1];
                if (trimmed.includes('{')) braceDepth++;
                continue;
            }

            // Block start (table, enum, embed)
            const blockMatch = trimmed.match(/^(table|enum|embed)\s+(\w+)\s*\{?/);
            if (blockMatch) {
                blockType = blockMatch[1];
                currentBlock = {
                    type: blockType,
                    name: blockMatch[2],
                    namespace: currentNamespace,
                    fullName: currentNamespace ? `${currentNamespace}.${blockMatch[2]}` : blockMatch[2],
                    fields: [],
                    values: []
                };
                blockContent = [];
                if (trimmed.includes('{')) braceDepth++;
                continue;
            }

            // Track braces
            const openBraces = (trimmed.match(/\{/g) || []).length;
            const closeBraces = (trimmed.match(/\}/g) || []).length;
            braceDepth += openBraces - closeBraces;

            // Block end
            if (currentBlock && trimmed.includes('}')) {
                if (currentBlock.type === 'table') {
                    tables.push(currentBlock);
                } else if (currentBlock.type === 'enum') {
                    enums.push(currentBlock);
                } else if (currentBlock.type === 'embed') {
                    embeds.push(currentBlock);
                }
                currentBlock = null;
                blockType = null;
                continue;
            }

            // Parse block content
            if (currentBlock) {
                if (currentBlock.type === 'enum') {
                    // Enum value: Name = value;
                    const enumMatch = trimmed.match(/^(\w+)\s*(?:=\s*(\d+))?\s*;?/);
                    if (enumMatch) {
                        currentBlock.values.push({
                            name: enumMatch[1],
                            value: enumMatch[2] || null
                        });
                    }
                } else {
                    // Field: name: Type constraints;
                    const fieldMatch = trimmed.match(/^(\w+)\s*:\s*(\w+)(\?)?(\[\])?\s*(.*?);?$/);
                    if (fieldMatch) {
                        const field = {
                            name: fieldMatch[1],
                            type: fieldMatch[2],
                            isOptional: !!fieldMatch[3],
                            isArray: !!fieldMatch[4],
                            constraints: parseConstraints(fieldMatch[5] || '')
                        };
                        currentBlock.fields.push(field);

                        // Detect relationships
                        if (field.constraints.foreign_key) {
                            relationships.push({
                                from: currentBlock.fullName,
                                fromField: field.name,
                                to: field.constraints.foreign_key.split('.')[0],
                                toField: field.constraints.foreign_key.split('.')[1] || 'id',
                                type: 'foreign_key'
                            });
                        } else if (!isPrimitiveType(field.type)) {
                            // Reference to another type
                            relationships.push({
                                from: currentBlock.fullName,
                                fromField: field.name,
                                to: field.type,
                                toField: null,
                                type: field.isArray ? 'has_many' : 'has_one'
                            });
                        }
                    }
                }
            }
        }

        // Resolve embed fields (inline expansion)
        for (const table of tables) {
            const expandedFields = [];
            for (const field of table.fields) {
                const embed = embeds.find(e => e.name === field.type || e.fullName === field.type);
                if (embed) {
                    // Inline embed fields with prefix
                    for (const embedField of embed.fields) {
                        expandedFields.push({
                            ...embedField,
                            name: `${field.name}.${embedField.name}`,
                            isEmbedded: true,
                            embedSource: field.name
                        });
                    }
                } else {
                    expandedFields.push(field);
                }
            }
            table.expandedFields = expandedFields;
        }

        return { tables, enums, embeds, relationships };
    }

    function parseConstraints(str) {
        const constraints = {};

        if (str.includes('primary_key')) constraints.primary_key = true;
        if (str.includes('unique')) constraints.unique = true;
        if (str.includes('index')) constraints.index = true;

        const fkMatch = str.match(/foreign_key\s*\(\s*([\w.]+)\s*\)/);
        if (fkMatch) constraints.foreign_key = fkMatch[1];

        const defaultMatch = str.match(/default\s*\(\s*([^)]+)\s*\)/);
        if (defaultMatch) constraints.default = defaultMatch[1];

        const maxLenMatch = str.match(/max_length\s*\(\s*(\d+)\s*\)/);
        if (maxLenMatch) constraints.max_length = parseInt(maxLenMatch[1]);

        const rangeMatch = str.match(/range\s*\(\s*(\d+)\s*,\s*(\d+)\s*\)/);
        if (rangeMatch) constraints.range = [parseInt(rangeMatch[1]), parseInt(rangeMatch[2])];

        return constraints;
    }

    function isPrimitiveType(type) {
        return [
            'string', 'bool', 'bytes',
            'u8', 'u16', 'u32', 'u64',
            'i8', 'i16', 'i32', 'i64',
            'f32', 'f64'
        ].includes(type);
    }

    // SVG Rendering
    function renderDiagram(schema, options = {}) {
        const {
            tableWidth = 220,
            rowHeight = 24,
            padding = 20,
            gap = 80,
            headerHeight = 32
        } = options;

        const { tables, enums, relationships } = schema;
        const allEntities = [...tables, ...enums];

        // Calculate positions (simple grid layout)
        const cols = Math.ceil(Math.sqrt(allEntities.length));
        const positions = new Map();

        allEntities.forEach((entity, i) => {
            const col = i % cols;
            const row = Math.floor(i / cols);
            const fields = entity.expandedFields || entity.fields || entity.values || [];
            const height = headerHeight + fields.length * rowHeight + padding;

            positions.set(entity.fullName || entity.name, {
                x: padding + col * (tableWidth + gap),
                y: padding + row * (headerHeight + 8 * rowHeight + gap),
                width: tableWidth,
                height: height,
                entity: entity
            });
        });

        // Calculate SVG size
        const maxX = Math.max(...Array.from(positions.values()).map(p => p.x + p.width)) + padding;
        const maxY = Math.max(...Array.from(positions.values()).map(p => p.y + p.height)) + padding;

        let svg = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 ${maxX} ${maxY}" class="poly-diagram">`;

        // Defs for arrows
        svg += `
            <defs>
                <marker id="arrow" markerWidth="10" markerHeight="10" refX="9" refY="3" orient="auto" markerUnits="strokeWidth">
                    <path d="M0,0 L0,6 L9,3 z" fill="var(--arrow-color, #888)"/>
                </marker>
                <marker id="arrow-many" markerWidth="12" markerHeight="12" refX="6" refY="6" orient="auto" markerUnits="strokeWidth">
                    <path d="M0,3 L6,6 L0,9 M6,0 L6,12" stroke="var(--arrow-color, #888)" fill="none" stroke-width="1.5"/>
                </marker>
            </defs>
        `;

        // Draw relationships first (behind tables)
        for (const rel of relationships) {
            const fromPos = positions.get(rel.from);
            const toPos = positions.get(rel.to) || positions.get(findEntityByName(allEntities, rel.to));

            if (fromPos && toPos) {
                svg += renderRelationship(fromPos, toPos, rel);
            }
        }

        // Draw tables and enums
        for (const [name, pos] of positions) {
            if (pos.entity.type === 'enum') {
                svg += renderEnum(pos);
            } else {
                svg += renderTable(pos);
            }
        }

        svg += '</svg>';
        return svg;
    }

    function findEntityByName(entities, name) {
        const entity = entities.find(e =>
            e.name === name ||
            e.fullName === name ||
            e.fullName?.endsWith('.' + name)
        );
        return entity?.fullName || entity?.name;
    }

    function renderTable(pos) {
        const { x, y, width, height, entity } = pos;
        const fields = entity.expandedFields || entity.fields || [];
        const headerHeight = 32;
        const rowHeight = 24;

        let svg = `
            <g class="table-group" transform="translate(${x}, ${y})">
                <rect class="table-bg" x="0" y="0" width="${width}" height="${height}" rx="6"/>
                <rect class="table-header" x="0" y="0" width="${width}" height="${headerHeight}" rx="6"/>
                <rect class="table-header-bottom" x="0" y="${headerHeight - 6}" width="${width}" height="6"/>
                <text class="table-name" x="${width/2}" y="21">${entity.name}</text>
                <line class="table-divider" x1="0" y1="${headerHeight}" x2="${width}" y2="${headerHeight}"/>
        `;

        fields.forEach((field, i) => {
            const fy = headerHeight + i * rowHeight + 17;
            const isPK = field.constraints?.primary_key;
            const isFK = field.constraints?.foreign_key;
            const isEmbedded = field.isEmbedded;

            // Field icon
            let icon = '';
            if (isPK) icon = '🔑';
            else if (isFK) icon = '🔗';
            else if (isEmbedded) icon = '📦';

            // Type display
            let typeStr = field.type;
            if (field.isOptional) typeStr += '?';
            if (field.isArray) typeStr += '[]';

            const fieldClass = isEmbedded ? 'field-embedded' : '';

            svg += `
                <text class="field-icon" x="8" y="${fy}">${icon}</text>
                <text class="field-name ${fieldClass}" x="28" y="${fy}">${field.name}</text>
                <text class="field-type" x="${width - 8}" y="${fy}">${typeStr}</text>
            `;
        });

        svg += '</g>';
        return svg;
    }

    function renderEnum(pos) {
        const { x, y, width, height, entity } = pos;
        const values = entity.values || [];
        const headerHeight = 32;
        const rowHeight = 24;

        let svg = `
            <g class="enum-group" transform="translate(${x}, ${y})">
                <rect class="enum-bg" x="0" y="0" width="${width}" height="${height}" rx="6"/>
                <rect class="enum-header" x="0" y="0" width="${width}" height="${headerHeight}" rx="6"/>
                <rect class="enum-header-bottom" x="0" y="${headerHeight - 6}" width="${width}" height="6"/>
                <text class="enum-label" x="8" y="21">enum</text>
                <text class="enum-name" x="${width/2}" y="21">${entity.name}</text>
                <line class="table-divider" x1="0" y1="${headerHeight}" x2="${width}" y2="${headerHeight}"/>
        `;

        values.forEach((val, i) => {
            const vy = headerHeight + i * rowHeight + 17;
            svg += `
                <text class="enum-value" x="12" y="${vy}">${val.name}</text>
                <text class="enum-number" x="${width - 8}" y="${vy}">${val.value || i}</text>
            `;
        });

        svg += '</g>';
        return svg;
    }

    function renderRelationship(fromPos, toPos, rel) {
        // Calculate connection points
        const fromCenterX = fromPos.x + fromPos.width / 2;
        const fromCenterY = fromPos.y + fromPos.height / 2;
        const toCenterX = toPos.x + toPos.width / 2;
        const toCenterY = toPos.y + toPos.height / 2;

        // Determine which sides to connect
        let x1, y1, x2, y2;

        if (Math.abs(fromCenterX - toCenterX) > Math.abs(fromCenterY - toCenterY)) {
            // Horizontal connection
            if (fromCenterX < toCenterX) {
                x1 = fromPos.x + fromPos.width;
                x2 = toPos.x;
            } else {
                x1 = fromPos.x;
                x2 = toPos.x + toPos.width;
            }
            y1 = fromCenterY;
            y2 = toCenterY;
        } else {
            // Vertical connection
            x1 = fromCenterX;
            x2 = toCenterX;
            if (fromCenterY < toCenterY) {
                y1 = fromPos.y + fromPos.height;
                y2 = toPos.y;
            } else {
                y1 = fromPos.y;
                y2 = toPos.y + toPos.height;
            }
        }

        // Curved path
        const midX = (x1 + x2) / 2;
        const midY = (y1 + y2) / 2;

        const relClass = rel.type === 'foreign_key' ? 'rel-fk' :
                         rel.type === 'has_many' ? 'rel-many' : 'rel-one';
        const marker = rel.type === 'has_many' ? 'url(#arrow-many)' : 'url(#arrow)';

        return `
            <path class="relationship ${relClass}"
                  d="M${x1},${y1} Q${midX},${y1} ${midX},${midY} Q${midX},${y2} ${x2},${y2}"
                  marker-end="${marker}"/>
        `;
    }

    // Public API
    return {
        parseSchema,
        renderDiagram,
        render(input, options) {
            const schema = parseSchema(input);
            return renderDiagram(schema, options);
        }
    };
})();

// Export for Node.js
if (typeof module !== 'undefined' && module.exports) {
    module.exports = PolyDiagram;
}
