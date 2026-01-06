SET @clinic = '苏州颜术苏悦广场店,杭州颜术坤和店,杭州颜术西子店,上海颜术汇银店,杭州颜术城西店,杭州颜术万通店,杭州颜术悦容医疗美容诊所,杭州颜术时尚医疗美容诊所,杭州颜术新芽医疗美容诊所,苏州颜术医疗美容诊所有限公司,杭州颜术素颜丰潭医疗美容诊所,上海颜术高得医疗美容门诊部';
-- 先删除已有表，防止重复创建
DROP TABLE IF EXISTS bi.wechat_cu_service;

-- 创建表结构
CREATE TABLE bi.wechat_cu_service (
    病历编号 VARCHAR(255),
    顾客姓名 VARCHAR(255),
    客户来源 VARCHAR(255),
    健康管理人 VARCHAR(255),
    健康管理人是否在职 VARCHAR(255),
    所属医生 VARCHAR(255),
    所属医生是否在职 VARCHAR(255),
    所属门店 VARCHAR(255),
    会员等级 VARCHAR(255)
);

-- 插入数据
INSERT INTO bi.wechat_cu_service (病历编号, 顾客姓名,客户来源, 健康管理人,健康管理人是否在职,所属医生,所属医生是否在职,所属门店,会员等级)


SELECT
    cu.CID AS `病历编号`,
    cu.Name AS `顾客姓名`,
    s.`Name` AS `客户来源`,
    emp.EmpName AS `健康管理人`,
		case WHEN emp.`Status` = 0 THEN'在职'
		     WHEN emp.`Status` = 1 THEN'离职'
				 END AS 健康管理人是否在职,
		emp2.EmpName AS `所属医生` ,
			case WHEN emp2.`Status` = 0 THEN'在职'
		     WHEN emp2.`Status` = 1 THEN'离职'
				 END AS 所属医生是否在职,
    org.`Name` AS 所属门店,
		vip.CardName AS 会员等级
FROM customers cu
LEFT JOIN employees emp ON cu.CustomerServerID = emp.ID
        AND  emp.ID NOT IN ('3527447c-213e-c923-6830-08dd5c84a6c5','17c9f191-5a2e-c061-5108-08dcf63900c2','f32d9a65-75e3-cecf-74c8-08dd4fca7e8d','edc32350-d5c1-cffa-7a64-08db6010c685')
LEFT JOIN organizations org on cu.OrganizationID = org.ID    
       AND cu.CID !='a000548'
       AND cu.ID !='2b6b02e0-d225-c6ba-a37a-08d631a6a080'
 LEFT JOIN employees emp2 ON cu.DoctorID =emp2.ID
 LEFT JOIN customercards  cuc
	ON cu.ID =cuc.ID
	LEFT JOIN vipcards vip
	ON cuc.CardID =vip.ID
  LEFT JOIN sources s
  ON cu.LaiYuanID = s.ID