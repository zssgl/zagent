SET @clinic = '苏州颜术苏悦广场店,杭州颜术坤和店,杭州颜术西子店,上海颜术汇银店,杭州颜术城西店,杭州颜术万通店,杭州颜术悦容医疗美容诊所,杭州颜术时尚医疗美容诊所,杭州颜术新芽医疗美容诊所,苏州颜术医疗美容诊所有限公司,杭州颜术素颜丰潭医疗美容诊所,上海颜术高得医疗美容门诊部';

-- 删除旧表（如果存在）
DROP TABLE IF EXISTS bi.wechat_bind_daily_cid;

-- 创建新表，使用中文字段名
CREATE TABLE bi.wechat_bind_daily_cid (
    `到访诊所` VARCHAR(255),
    `customerid` VARCHAR(50),
    `到访时间` DATE,
    `病历编号` VARCHAR(50),
    `健康管理人` VARCHAR(255),
    `姓名` VARCHAR(255)
);

-- 插入数据，确保字段名与表结构完全匹配
INSERT INTO bi.wechat_bind_daily_cid (`到访诊所`, `customerid`, `到访时间`, `病历编号`, `健康管理人`, `姓名`)
SELECT
  v.OrginizationName,
  v.CustomerId,
  DATE(v.StartTime),
  cu.CID,
  emp.EmpName,
  cu.`Name`
FROM visits v
  LEFT JOIN customers cu
  ON v.CustomerId = cu.ID
  AND cu.CID != '02154340'
  LEFT JOIN employees emp
  ON cu.CustomerServerID = emp.ID
  AND emp.ID NOT IN ('3527447c-213e-c923-6830-08dd5c84a6c5','17c9f191-5a2e-c061-5108-08dcf63900c2','f32d9a65-75e3-cecf-74c8-08dd4fca7e8d','edc32350-d5c1-cffa-7a64-08db6010c685')
WHERE FIND_IN_SET(v.OrginizationName, @clinic)
  AND v.`Status` != 0
	AND cu.`Name` NOT LIKE "%QW%"
  -- AND DATE(v.StartTime) >= '2024-01-01'