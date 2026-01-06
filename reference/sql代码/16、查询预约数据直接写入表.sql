DROP TABLE IF EXISTS bi.appoint_detail;

CREATE TABLE bi.appoint_detail (
    预约日期 DATE,
    预约诊所 VARCHAR(100),
    病历编号 VARCHAR(50),
    预约状态 INT
);
INSERT INTO bi.appoint_detail (预约日期, 预约诊所, 病历编号, 预约状态)
SELECT
    DATE_FORMAT(a.StartTime, '%Y-%m-%d') AS `预约日期`,
    a.OrginizationName AS `预约诊所`,
    cu.CID AS `病历编号`,
    a.Status AS `预约状态`
FROM appointments a
LEFT JOIN customers cu
ON a.CustomerId = cu.ID
WHERE a.StartTime >= '2024-01-01'

UNION ALL

SELECT
    DATE_FORMAT(消耗时间, '%Y-%m-%d') AS `预约日期`,
    消耗诊所 AS `预约诊所`,
    病历编号 AS `病历编号`,
    '1' AS `预约状态`
FROM bi.report_consumption_detail 
WHERE  消耗时间 >= '2024-01-01'