| bit_offset | bit_length | description                                                                  | data_type | units |
|------------|------------|------------------------------------------------------------------------------|-----------|-------|
| 0          | 8          | SID - The sequence identifier field is used to tie related PGNs together     | UINT      |       |
| 8          | 10         | Command angle (deg, unused for Yaw)                                          | Scaled    | deg   |
| 18         | 10         | Angle P (unused for Yaw)                                                     | Scaled    | deg/s |
| 28         | 10         | Angle I (unused for Yaw)                                                     | Scaled    | deg/s |
| 38         | 10         | Angle D (unused for Yaw)                                                     | Scaled    | deg/s |
| 48         | 10         | Rate Command (output of Angle PID for Roll/Pitch, direct Rate input for Yaw) | Scaled    | deg/s |
| 58         | 10         | Rate P - Proportional gain for rate control                                  | Q9        |       |
| 68         | 10         | Rate I - Integral gain for rate control                                      | Q9        |       |
| 78         | 10         | Rate D - Derivative gain for rate control                                    | Q9        |       |
| 88         | 10         | PID Output - Final control output value                                      | Q9        |       |
| 98         | 2          | Axis selection (Roll, Pitch, Yaw, or Undefined)                              | Enum      |       |
| 104        | 8          | Saturation Mask - Indicates which PID terms are saturated                    | UINT      |       |
|            |            | This is a multiline doc                                                      |           |       |