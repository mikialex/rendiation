# 3D Math primitives

## Intersections

<!-- https://www.tablesgenerator.com/markdown_tables# -->

|         | ray | box3                    | face3 | sphere                  | frustum |
|---------|-----|-------------------------|-------|-------------------------|---------|
| ray     |     | ifHit, HitPointNearesrt |       | ifHit, HitPointNearesrt |         |
| box3    | /   |                         |       |                         |         |
| face3   | /   | /                       |       |                         |         |
| sphere  | /   | /                       | /     |                         | ifHit   |
| frustum | /   | /                       | /     | /                       |         |
